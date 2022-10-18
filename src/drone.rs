#![allow(dead_code,clippy::redundant_field_names)]

use std::{sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}, collections::VecDeque, time::Duration};

use tokio::{net::UdpSocket, task::JoinHandle, sync::Mutex};

pub const ACKED: &[&str] = &[];

pub struct SdkCommand {
	pub cmd: String, // the formatted command to run for the drone
	pub blocking: bool //whether or not this command can be "locking" which means you can wait for *all* drones to finish their last locking command
}


pub struct Drone {
	pub id: String,
	queue: Arc<Mutex<VecDeque<SdkCommand>>>,
	drn_ack: Arc<AtomicBool>,
	command_sock: Arc<UdpSocket>,
	command_resp: Arc<Mutex<String>>,
	command_thread: JoinHandle<()>,
	send_thread: JoinHandle<()>,
	block_counter: Arc<AtomicUsize>
}

impl Drone {
	pub async fn connect(addr: &str) -> Drone {
		let response = Arc::new(Mutex::new(String::new()));
		let acked = Arc::new(AtomicBool::new(false));
		let udp_socket = Arc::new(UdpSocket::bind(addr).await.unwrap());
		let command_queue = Arc::new(Mutex::new(VecDeque::<SdkCommand>::new()));

		let rsp = response.clone();
		let ack = acked.clone();
		let sck = udp_socket.clone();
		let command_thread = tokio::spawn(async move { loop {
			let mut buffer: [u8; 1500] = [0; 1500];
            {
                let result = sck.recv_from(&mut buffer).await;
                if let Ok((size,_)) = result {
                let response = std::str::from_utf8(&buffer[..size]).unwrap().trim();
					*rsp.lock().await = response.to_string();
                    ack.store(true, Ordering::SeqCst)
                }
            }
            std::thread::sleep(Duration::from_millis(20));
		}});

		let sck = udp_socket.clone();
		let queue = command_queue.clone();
		let address = addr.to_string();
		let send_thread = tokio::spawn(async move {
            loop {
				{
					let mut uqueue = queue.lock().await;
					if !uqueue.is_empty() {
						let command = uqueue.pop_front().unwrap();
						sck.send_to(command.cmd.as_bytes(), address.clone()).await;
					}
				}
				std::thread::sleep(Duration::from_millis(20));
			}
		});

		Drone {
			id: addr.to_string(),
			queue: command_queue,
			drn_ack: acked.clone(),
			command_thread: command_thread,
			command_sock: udp_socket,
			command_resp: response,
			block_counter: Arc::new(AtomicUsize::new(0)),
			send_thread: send_thread
		}
	}
	pub async fn add_command(&self,cmd: SdkCommand) {
		if cmd.blocking {
			let t = self.block_counter.load(Ordering::Relaxed);
			self.block_counter.store(t+1,Ordering::Relaxed);
		}
		let mut queu = self.queue.lock().await;
		queu.push_back(cmd);
	}
	pub fn await_blocks(&self) {
		let mut wait_blocks = self.block_counter.load(Ordering::Relaxed);
		while wait_blocks > 0 {
			std::thread::sleep(Duration::from_millis(20));
			wait_blocks = self.block_counter.load(Ordering::Relaxed);
		}
	}
}
