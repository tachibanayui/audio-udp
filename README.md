# Audio UDP
This project is part of my learning playground about Rust and Windows API.
None of the code in this repository is suited for production use! 
Use this as a reference for your own adventure

## workspace items
### server 
Currently a udp server for send / receive wav data from / to an audio device

### program capture
Uses [ActivateAudioInterfaceAsync](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync) function to capture audio from a specific program.
Useful resources: 
- Audio client `GetMixFormat` returns `not implemented` [LoopbackCapture GetMixFormat failed with E_NOTIMPL](https://learn.microsoft.com/en-us/answers/questions/1125409)
- C++ Example: [Windows-classic-samples](https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/ApplicationLoopback/cpp/LoopbackCapture.cpp)
- Loopback Recording: https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording
- `window-rs` binding: https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Media/Audio/fn.ActivateAudioInterfaceAsync.html
