use std::{
    any::Any,
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};

pub struct ConsoleStatus {
    name: String,
    latest_data: Vec<(String, String)>,
    receiver: Receiver<(String, String)>,
    sender: Sender<(String, String)>,
    report_rate: Duration,
}

impl ConsoleStatus {
    pub fn new(name: String, report_rate: Duration) -> Self {
        let (send, recv) = unbounded();

        Self {
            name,
            latest_data: Vec::new(),
            sender: send,
            receiver: recv,
            report_rate,
        }
    }

    pub fn start(self) -> (JoinHandle<()>, ConsoleStatusHandle) {
        let sender = self.sender.clone();
        let thd = thread::spawn(|| self.process_loop());
        (thd, ConsoleStatusHandle { sender })
    }

    fn process_loop(mut self) {
        loop {
            thread::sleep(self.report_rate);
            self.update_report().unwrap();
            self.write();
        }
    }

    pub fn write(&self) {
        let mut fin = self.name.clone();
        for (name, value) in &self.latest_data {
            fin.push_str(" | ");
            fin.push_str(&name);
            fin.push_str(": ");
            fin.push_str(&value);
        }

        println!("{}", fin);
    }

    pub fn update_report(&mut self) -> Result<(), RecvError> {
        while !self.receiver.is_empty() {
            let (n, v) = self.receiver.recv()?;
            self.report(n, v);
        }

        Ok(())
    }

    pub fn report(&mut self, name: String, value: String) {
        for (n, v) in &mut self.latest_data {
            if name.eq(n) {
                *v = value;
                return;
            }
        }

        self.latest_data.push((name, value));
    }
}

#[derive(Clone)]
pub struct ConsoleStatusHandle {
    sender: Sender<(String, String)>,
}


impl ConsoleStatusHandle {
    pub fn report(
        &self,
        name: String,
        value: String,
    ) -> Result<(), crossbeam_channel::SendError<(String, String)>> {
        self.sender.send((name, value))
    }
}

#[test]
fn mx() {
    let reporter = ConsoleStatus::new("Test".into(), Duration::from_millis(10));
    let (join, handle) = reporter.start();
    handle.report("Hello".into(), "world".into()).unwrap();
    handle.report("xd".into(), "dx".into()).unwrap();
    join.join();
}
