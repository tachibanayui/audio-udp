pub mod utils;

use std::{net::UdpSocket, mem, f32::consts::PI, thread, time::Duration};

// use crossbeam_channel::{bounded, Receiver, unbounded};
// use utils::{to_reference_time, SigoidWaveIter};
// use windows::Win32::{
//     Media::Audio::{
//         eConsole, eRender, EDataFlow, IAudioClient3, IAudioRenderClient, IMMDeviceEnumerator,
//         MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, WAVEFORMATEXTENSIBLE,
//     },
//     System::Com::{
//         CoCreateInstance, CoInitialize, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
//     },
// };

pub struct SigoidWaveIter {
    sample_rate: u32,

    sample_clock: u32,
    c_freq: f32,
}

impl SigoidWaveIter {
    pub fn new(sample_rate: u32, c_freq: f32) -> Self {
        Self {
            sample_rate,
            sample_clock: 0,
            c_freq,
        }
    }
}

impl Iterator for SigoidWaveIter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.sample_clock += 1;
        self.sample_clock %= self.sample_rate;

        let next =
            (self.sample_clock as f32 * self.c_freq * 2.0 * PI / self.sample_rate as f32).sin();
        Some(next)
    }
}


fn main() {
    unsafe {
        let sk = UdpSocket::bind("0.0.0.0:0").unwrap();
        let mut sig = SigoidWaveIter::new(48000, 1000f32);
        
        loop {
            for x in 0..100 {
                let mut data = [0f32; 128];
                for x in data.chunks_mut(2) {
                    let new = sig.next().unwrap();
                    for c in x {
                        *c = new;
                    }
                }
    
                let buf = std::slice::from_raw_parts_mut(&mut data as *mut f32 as *mut u8, data.len() * mem::size_of::<f32>());
                let a = sk.send_to(buf, "127.0.0.1:16969").unwrap();
            }
            thread::sleep(Duration::from_millis(100));
        }

    }
    
}


// fn start_send() {
//     let TIME = Duration::from_millis(22);
//     CoInitializeEx(None, COINIT_MULTITHREADED)?;
//     let mmd_enum: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
//     let out_dev = mmd_enum.GetDefaultAudioEndpoint(eRender, eConsole)?;
//     let client: IAudioClient3 = out_dev.Activate(CLSCTX_ALL, None)?;
//     let mix_format = client.GetMixFormat()?;
//     let mix_format_ex = mix_format as *mut WAVEFORMATEXTENSIBLE;
//     let channel_count = (*mix_format).nChannels;
//     client.Initialize(
//         AUDCLNT_SHAREMODE_SHARED,
//         0,
//         to_reference_time(TIME * 2),
//         0,
//         mix_format,
//         None,
//     )?;
//     let bf_size = client.GetBufferSize()?;
// }