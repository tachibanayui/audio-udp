pub mod activate_ex;

use std::{error::Error, time::Duration};

use activate_ex::activate_audio_interface_async;
use windows::{
    core::ComInterface,
    Win32::{
        Media::Audio::{
            eConsole, eRender, ActivateAudioInterfaceAsync, EDataFlow, IAudioCaptureClient,
            IAudioClient, IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS,
            AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, WAVEFORMATEX,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, StructuredStorage::PROPVARIANT, CLSCTX,
                CLSCTX_ALL, COINIT_MULTITHREADED,
            },
            Variant::VT_BLOB,
        },
    },
};

struct Hello {
    v: Vec<i32>,
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    unsafe {
        win_main().await.unwrap();
    }
}

const TIME: Duration = Duration::from_millis(2);

async unsafe fn win_main() -> Result<(), Box<dyn Error>> {
    CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();

    let mut params = AUDIOCLIENT_ACTIVATION_PARAMS::default();
    params.ActivationType = AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK;
    params.Anonymous.ProcessLoopbackParams.ProcessLoopbackMode =
        PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE;
    params.Anonymous.ProcessLoopbackParams.TargetProcessId = 78884;
    let pv = into_propvariant(&params);
    let aud_client: IAudioClient =
        activate_audio_interface_async(VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, Some(&pv))
            .await
            .unwrap();





    let enummerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
    let def_dev = enummerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .unwrap();
    let client: IAudioClient = def_dev.Activate(CLSCTX_ALL, None).unwrap();
    let fmt_ref = client.GetMixFormat().unwrap();

    let f2 = get_format_of_defdev();

    // This client doesn't support getmixformat()! https://learn.microsoft.com/en-us/answers/questions/1125409/
    aud_client
        .Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            to_reference_time(TIME * 2),
            0,
            fmt_ref,
            None,
        )
        .unwrap();
    let bf_size = aud_client.GetBufferSize().unwrap();
    dbg!(bf_size);
    let capture_client: IAudioCaptureClient = aud_client.GetService().unwrap();
    aud_client.Start().unwrap();

    loop {
        std::thread::sleep(TIME / 2);
        let mut next_frame_count = capture_client.GetNextPacketSize().unwrap();

        while next_frame_count != 0 {
            let mut buf: *mut u8 = std::ptr::null_mut();
            let mut read_frames_count: u32 = 0;
            let mut flags = 0;
            capture_client
                .GetBuffer(&mut buf, &mut read_frames_count, &mut flags, None, None)
                .unwrap();
            let buf_f32 = std::slice::from_raw_parts(
                buf as *const u8 as *const f32,
                read_frames_count as usize * 2 as usize,
            );

            let a = buf_f32.iter().map(|x| (*x * 100000.0) as i32).max();
            println!("Read {:?} frames", a);

            capture_client.ReleaseBuffer(read_frames_count).unwrap();
            next_frame_count = capture_client.GetNextPacketSize().unwrap();
        }
    }

    Ok(())
}

unsafe fn get_format_of_defdev() -> WAVEFORMATEX {
    let enummerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
    let def_dev = enummerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .unwrap();
    let client: IAudioClient = def_dev.Activate(CLSCTX_ALL, None).unwrap();
    let fmt_ref = client.GetMixFormat().unwrap();

    let mut fmt = WAVEFORMATEX::default();
    fmt = *fmt_ref;
    fmt
}

/// `PROPVARIANT` referencing `AUDIOCLIENT_ACTIVATION_PARAMS`! Do not drop it before the returned value
unsafe fn into_propvariant(client: &AUDIOCLIENT_ACTIVATION_PARAMS) -> PROPVARIANT {
    use std::{mem, ptr};

    let mut p = PROPVARIANT::default();
    // let vt = &p.Anonymous.Anonymous.vt as *const _ as *mut _;
    // let blob_size = &p.Anonymous.Anonymous.Anonymous.blob.cbSize as *const _ as *mut _;
    // let blob_data = &p.Anonymous.Anonymous.Anonymous.blob.pBlobData as *const _ as *mut AUDIOCLIENT_ACTIVATION_PARAMS;

    unsafe {
        (*p.Anonymous.Anonymous).vt = VT_BLOB;
        (*p.Anonymous.Anonymous).Anonymous.blob.cbSize =
            mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32;
        (*p.Anonymous.Anonymous).Anonymous.blob.pBlobData = client as *const _ as *mut u8;
        p
    }
}

pub fn to_reference_time(d: Duration) -> i64 {
    (d.as_nanos() / 100) as i64
}
