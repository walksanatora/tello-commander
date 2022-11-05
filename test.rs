
use std::time::Duration;

#[derive(Debug)]
pub struct SdkCommand {
	pub cmd: String, // the formatted command to run for the drone
	pub blocking: bool //whether or not this command can be "locking" which means you can wait for *all* drones to finish their last locking command
}

struct ExDrone {id:u8}
impl ExDrone{
	fn await_blocks(&self) {
		println!("waiting drone {}",self.id)
	}
	fn add_command(&self,cmd: SdkCommand) {
		println!("({}) running {:?}",self.id,cmd)
	}
}


fn main(){
	let drones: Vec<ExDrone> = vec![
		ExDrone{id:0},ExDrone{id:1},ExDrone{id:2}
	];
	let code = include_str!("example.ds");
	for command in code.split('\n') {
		if command.starts_with('#') || command.is_empty() || command  == "\n" {
			println!("comment/blank line");
			continue
		}
		if command.starts_with("delay") {
			println!("delay");
			if let Some(num) = command.split(' ').nth(2){
				let n = num.to_string().parse::<usize>();
				if let Ok(nm) = n {std::thread::sleep(Duration::from_secs(nm as u64))}
				else {std::thread::sleep(Duration::from_secs(1))};
			} else {std::thread::sleep(Duration::from_secs(1))}
			continue
		} else if command.starts_with("await") {
			for drone in drones.iter() {
				drone.await_blocks();
			}
		}

		let is_blocking = command.contains('@');
		if command.contains('>') {
			// push command to one drones queue
			let split: Vec<&str> = command.splitn(2,'>').collect();
			let num = split[0].chars().filter(|x|{x.is_alphanumeric()||x==&' '}).collect::<String>().parse::<usize>().unwrap();
			let comma: String = split[1].chars().filter(|x|{x.is_alphanumeric()||x.is_whitespace()}).collect();
			if num < drones.len(){
				drones[num].add_command(SdkCommand{
					cmd: comma,
					blocking: is_blocking
				});
			};
		} else {
			// push commands to all drones queue
			let comma: String = command.chars().filter(|x|{x.is_alphanumeric()||x==&' '}).collect();
			for drone in drones.iter() {
				drone.add_command(SdkCommand{
					cmd: comma.clone(),
					blocking: is_blocking
				});
			};
		};

	}
}