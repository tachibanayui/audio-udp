pub mod enc;

use std::{collections::VecDeque, error::Error, mem::MaybeUninit, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host, Stream, StreamConfig,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use opus::{Bitrate, Decoder, Encoder};
use ringbuf::{LocalRb, Rb};

/// OPUS Config - Fine tune if you like
const OPUS_APP: opus::Application = opus::Application::LowDelay;
// complexity is a value from 1 to 10, where 1 is the lowest complexity and 10 is the highest
const OPUS_COMPLEXITY: i32 = 10;
const OPUS_BITRATE: Bitrate = Bitrate::Bits(5000);
const OPUS_FEC: bool = true;

const STREAM_CFG: StreamConfig = StreamConfig {
    buffer_size: cpal::BufferSize::Default,
    channels: 2,
    sample_rate: cpal::SampleRate(48000),
};

fn main() {
    println!("Hello, world!");
    let (send, recv) = unbounded();
    let host = cpal::default_host();

    // stream closed if not assigned to a variable!
    let buffer_duration = Duration::from_millis(20);
    let _in_stream = initalize_input2(&host, send, OPUS_BITRATE, OPUS_COMPLEXITY, STREAM_CFG, buffer_duration).unwrap();
    let _out_stream = initalize_output(&host, recv, OPUS_BITRATE, buffer_duration, STREAM_CFG).unwrap();
    loop {}
    // std::thread::sleep(Duration::from_secs(60));
}

fn duration_to_frame_count(sample_rate: u32, duration: Duration) -> u32 {
    sample_rate * duration.as_millis() as u32 / 1000  as u32
}

fn create_in_buf<T: Default + Copy>(channel_count: u32, requested_frames_count: u32) -> Box<[T]> {
    let size = channel_count * requested_frames_count;
    vec![T::default(); size as usize].into_boxed_slice()
}

fn create_opus_buf(bitrate: u32, duration: Duration) -> Box<[u8]> {
    let size = (duration.as_millis() as u32) * bitrate / 1000 / 8;
    vec![0; size as usize].into_boxed_slice()
}

fn initalize_input2(
    host: &Host,
    send: Sender<u8>,
    bitrate: Bitrate,
    complexity: i32,
    config: StreamConfig,
    buffer_duration: Duration,
) -> Result<Stream, Box<dyn Error>> {
    let dev = host
        .default_input_device()
        .ok_or("Error get default device")?;
    let name = dev.name()?;

    let sample_rate = config.sample_rate.0;
    let mut enc = Encoder::new(sample_rate, opus::Channels::Stereo, OPUS_APP)?;
    enc.set_complexity(complexity)?;
    enc.set_bitrate(bitrate)?;
    enc.set_inband_fec(OPUS_FEC)?;
    enc.set_vbr(true)?;

    let mut buf: Box<[f32]> = create_in_buf(
        2,
        duration_to_frame_count(sample_rate, buffer_duration),
    );
    let mut buf_idx = 0;

    let Bitrate::Bits(bit) = bitrate else {
        panic!();
    };

    let mut opus_buf = create_opus_buf(bit as u32, buffer_duration);
    let stream = dev.build_input_stream(
        &STREAM_CFG,
        move |data: &[f32], _| {
            let mut data_idx = 0;

            while data.len() - data_idx > 0 {
                let remaining_data = data.len() - data_idx;
                let remaining_buf = buf.len() - buf_idx;
                let copiable = usize::min(remaining_buf, remaining_data);
                buf[buf_idx..(buf_idx + copiable)].copy_from_slice(&data[data_idx..(data_idx + copiable)]);
                buf_idx += copiable;
                data_idx += copiable;

                if buf_idx == buf.len() {
                    let written_bytes = enc.encode_float(&buf, &mut opus_buf).unwrap();
                    for idx in 0..written_bytes {
                        send.send(opus_buf[idx]).unwrap();
                    }
                    buf_idx = 0;
                    // println!("+ {} / {}", written_bytes, send.len());
                }
            }
        },
        |e| {
            dbg!(e);
        },
        Some(Duration::from_secs(1)),
    )?;
    stream.play()?;
    println!("Done init input device: {}!", name);
    Ok(stream)
}

fn initalize_output(host: &Host, recv: Receiver<u8>, bitrate: Bitrate, buffer_duration: Duration, config: StreamConfig) -> Result<Stream, Box<dyn Error>> {
    let dev = host
        .default_output_device()
        .ok_or("Error get default device")?;
    let mut dnc = Decoder::new(config.sample_rate.0, opus::Channels::Stereo)?;

    let Bitrate::Bits(bits) = bitrate else {
        panic!("gg");
    };

    let mut opus_buf = create_opus_buf(bits as u32, buffer_duration);
    let mut opus_buf_idx = 0;
    let mut buf = create_in_buf(2, duration_to_frame_count(config.sample_rate.0, buffer_duration));

    let mut rb: LocalRb<f32, Vec<MaybeUninit<f32>>> = LocalRb::new(buf.len());
    let stream = dev.build_output_stream(
        &STREAM_CFG,
        move |data: &mut [f32], _| {
            if rb.len() == 0 {
                while let Ok(x) = recv.try_recv() {
                    opus_buf[opus_buf_idx] = x; 
                    opus_buf_idx += 1;
    
                    if opus_buf_idx == opus_buf.len() {
                        let _written_frames = dnc.decode_float(&opus_buf, &mut buf, false).unwrap();
                        rb.push_slice_overwrite(&buf);
                        opus_buf_idx = 0;
                        break;
                    }                
                }
            }

            for x in 0..(data.len()) {
                let i = rb.pop().unwrap_or(0.0);
                data[x] = i;
            }
        },
        |_| {},
        None,
    )?;
    stream.play()?;
    println!("Done init output device: {}!", dev.name()?);
    Ok(stream)
}
