pub mod utils;

use std::{error::Error, f32::consts::PI, mem::size_of, net::UdpSocket, time::Duration};

use crossbeam_channel::{bounded, Receiver, unbounded};
use crossbeam_queue::ArrayQueue;
use utils::{to_reference_time, SigoidWaveIter};
use windows::Win32::{
    Media::Audio::{
        eConsole, eRender, EDataFlow, IAudioClient3, IAudioRenderClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, WAVEFORMATEXTENSIBLE,
    },
    System::Com::{
        CoCreateInstance, CoInitialize, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};

enum Mode {
    Sender,
    Receiver,
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args
        .get(0)
        .map(|x| {
            if x == "send" {
                Mode::Sender
            } else {
                Mode::Receiver
            }
        })
        .unwrap_or(Mode::Receiver);

    let ip = args.get(1).map(|x| x.as_str()).unwrap_or("0.0.0.0:16969");

    unsafe {
        match mode {
            Mode::Receiver => recv().unwrap(),
            _ => (),
        }
    }
}

unsafe fn recv() -> Result<(), Box<dyn Error>> {
    let sk = UdpSocket::bind("0.0.0.0:16969")?;
    let mut buf = [0; 512];
    let (sender, receiver) = unbounded();

    let thd = std::thread::spawn(move || start_render(receiver));

    loop {
        let read = sk.recv(&mut buf)?;
        let buf_f32 = std::slice::from_raw_parts_mut(
            &mut buf as *mut u8 as *mut f32,
            read / size_of::<f32>(),
        );

        for x in buf_f32 {
            sender.send(*x)?;
            // inc_queue.force_push(*x);
        }
    }

    thd.join();
    return Ok(());
}

const REFTIMES_PER_SEC: i64 = 10000000;
const REFTIMES_PER_MILLISEC: i64 = 10000;
unsafe fn start_render(inc_queue: Receiver<f32>) -> windows::core::Result<()> {
    let TIME = Duration::from_millis(22);
    CoInitializeEx(None, COINIT_MULTITHREADED)?;
    let mmd_enum: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let out_dev = mmd_enum.GetDefaultAudioEndpoint(eRender, eConsole)?;
    let client: IAudioClient3 = out_dev.Activate(CLSCTX_ALL, None)?;
    let mix_format = client.GetMixFormat()?;
    let mix_format_ex = mix_format as *mut WAVEFORMATEXTENSIBLE;
    let channel_count = (*mix_format).nChannels;
    client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        0,
        to_reference_time(TIME * 2),
        0,
        mix_format,
        None,
    )?;
    let bf_size = client.GetBufferSize()?;
    dbg!(bf_size);
    let render_client: IAudioRenderClient = client.GetService()?;
    let mut sigoid = SigoidWaveIter::new(48000, 1000f32);
    let buf = render_client.GetBuffer(bf_size)?;
    let buf_f32 = std::slice::from_raw_parts_mut(
        buf as *mut f32,
        bf_size as usize * (channel_count as usize),
    );

    for x in buf_f32.chunks_mut(channel_count as usize) {
        let sample = sigoid.next().unwrap();
        for channel in x {
            *channel = sample;
        }
    }

    render_client.ReleaseBuffer(bf_size, 0)?;
    client.Start()?;

    loop {
        std::thread::sleep(TIME / 2);
        let padding = client.GetCurrentPadding()?;
        let writable_frames = bf_size - padding;
        // should be n_channel but we gonna remove other channels
        let requested_frame = writable_frames.min(inc_queue.len() as u32 / 2);

        let buf = render_client.GetBuffer(requested_frame)?;
        println!("{} | {}", requested_frame, inc_queue.len());
        let buf_f32 = std::slice::from_raw_parts_mut(
            buf as *mut f32,
            requested_frame as usize * (channel_count as usize),
        );

        for x in buf_f32 {
            if let Ok(inc) = inc_queue.try_recv() {
                *x = inc;
            }
        }

        render_client.ReleaseBuffer(requested_frame, 0)?;
    }

    return Ok(());
}
