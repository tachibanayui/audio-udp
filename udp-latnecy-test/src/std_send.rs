use std::net::UdpSocket;

use crate::ConfigArgs;

pub fn std_send(config: ConfigArgs) {
    let sk = UdpSocket::bind("0.0.0.0:0").unwrap();
    sk.connect(config.host);
    loop {
        
    }
}