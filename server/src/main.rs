use std::{
    error::Error,
    mem::{self, size_of},
    net::UdpSocket,
    thread,
    time::{Duration, Instant},
};

use common::{to_reference_time, ConsoleStatus, SigoidWaveIter, ThroughputMeter, ConsoleStatusHandle};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use windows::Win32::{
    Media::Audio::{
        eCapture, eConsole, eRender, EDataFlow, IAudioCaptureClient, IAudioClient3,
        IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED,
        WAVEFORMATEXTENSIBLE,
    },
    System::Com::{
        CoCreateInstance, CoInitialize, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};

#[derive(Debug, Clone, Copy)]
enum Mode {
    Sender,
    Receiver,
}

fn main2() {
    let args: Vec<_> = std::env::args().collect();
    dbg!(args.clone());
    let mode = args
        .get(1)
        .map(|x| {
            if x == "send" {
                Mode::Sender
            } else {
                Mode::Receiver
            }
        })
        .unwrap_or(Mode::Sender);

    let ip: String = args
        .get(2)
        .map(|x| x.clone())
        .unwrap_or(if let Mode::Receiver = mode {
            String::from("0.0.0.0:16969")
        } else {
            String::from("localhost:16969")
        });

    dbg!(mode, ip.clone());

    unsafe {
        match mode {
            Mode::Receiver => recv(ip).unwrap(),
            Mode::Sender => send(ip).unwrap(),
        }
    }
}

unsafe fn recv(ip: String) -> Result<(), Box<dyn Error>> {
    let sk = UdpSocket::bind(ip)?;
    let mut buf = [0; 512];
    let (sender, receiver) = unbounded();
    let report = ConsoleStatus::new("Recv".into(), Duration::from_millis(50));
    let (_, handle) = report.start();
    let mut counter = 0;
    let ins = Instant::now();
    let thd_handle = handle.clone();
    let thd = std::thread::spawn(move || start_render(receiver, &thd_handle));
    
    // let thruput = std::thread::spawn(move || {
    //     let recv = receiver2;

    //     loop {
    //         let x = recv.recv().unwrap();
    //         counter += 4;

    //         if counter % 10000 == 0 {
    //             let ela = ins.elapsed().as_millis();
    //             if ela > 0  {
    //                 let thru = counter * 1000 / ela;
    //                 println!("thru: {}Kbps", thru / 1000);
    //             }
    //         }
    //     }
    // });

    loop {
        let read = sk.recv(&mut buf)?;
        let buf_f32 = std::slice::from_raw_parts_mut(
            &mut buf as *mut u8 as *mut f32,
            read / size_of::<f32>(),
        );

        for x in buf_f32 {
            sender.send(*x)?;

            counter += 4;
            let ela = ins.elapsed().as_millis();
            if ela > 0 {
                let thru = counter * 1000 / ela;
                handle.report("thru".into(), format!("{}Kbps", thru / 1000)).unwrap();
            }
        }
    }

    thd.join();
    return Ok(());
}

const TIME: Duration = Duration::from_millis(2);

const REFTIMES_PER_SEC: i64 = 10000000;
const REFTIMES_PER_MILLISEC: i64 = 10000;
unsafe fn start_render(inc_queue: Receiver<f32>, report: &ConsoleStatusHandle) -> windows::core::Result<()> {
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
        report.report("render".into(), format!("{requested_frame} | {}", inc_queue.len())).unwrap();
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

unsafe fn send(ip: String) -> Result<(), Box<dyn Error>> {
    let (sender, recver) = unbounded();

    let sk = UdpSocket::bind("0.0.0.0:0").unwrap();
    sk.connect(ip)?;

    let thd = std::thread::spawn(move || {
        start_capture(sender).unwrap();
    });

    loop {
        let mut data = [0f32; 48 * 2 / 4];
        for i in 0..data.len() {
            let x = recver.recv()?;
            data[i] = x;
        }

        let buf = std::slice::from_raw_parts_mut(
            &mut data as *mut f32 as *mut u8,
            data.len() * mem::size_of::<f32>(),
        );
        sk.send(&buf)?;
    }
}

unsafe fn start_capture(outg_queue: Sender<f32>) -> windows::core::Result<()> {
    CoInitializeEx(None, COINIT_MULTITHREADED)?;
    let mmd_enum: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let out_dev = mmd_enum.GetDefaultAudioEndpoint(eCapture, eConsole)?;
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
    let capture_client: IAudioCaptureClient = client.GetService()?;
    client.Start()?;

    loop {
        thread::sleep(TIME / 2);
        let mut next_frame_count = capture_client.GetNextPacketSize()?;

        while next_frame_count != 0 {
            let mut buf: *mut u8 = std::ptr::null_mut();
            let mut read_frames_count: u32 = 0;
            let mut flags = 0;
            capture_client.GetBuffer(&mut buf, &mut read_frames_count, &mut flags, None, None)?;
            let buf_f32 = std::slice::from_raw_parts(
                buf as *const u8 as *const f32,
                (read_frames_count as usize * channel_count as usize),
            );

            for x in buf_f32 {
                outg_queue.send(*x).unwrap();
            }

            println!(
                "capture: {} | {} | {}",
                read_frames_count,
                outg_queue.len(),
                flags
            );
            capture_client.ReleaseBuffer(read_frames_count)?;
            next_frame_count = capture_client.GetNextPacketSize()?;
        }
    }

    return Ok(());
}

#[async_std::main]
async fn main() {
    main2()
}
