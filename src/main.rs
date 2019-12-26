fn is_checksum_valid(message: &Vec<zmq::Message>) -> bool {
	true
}

fn validate_message(message : Vec<zmq::Message>) -> zmq::Result<Vec<zmq::Message>> {
	if message.len() != 8 {
		return Err(zmq::Error::EMSGSIZE);
	} else if message[5].len() != 2 {
		return Err(zmq::Error::EPROTO);
	} else if message[5][1] > 3 {
		return Err(zmq::Error::ENOTSUP);
	} else if !is_checksum_valid(&message) {
		return Err(zmq::Error::EINVAL);
	}
	
	Ok(message)
}

fn receive_multi(socket : &zmq::Socket) -> Vec<zmq::Message> {
	let mut frame = Vec::new();
	let mut more = true;

	while more {
		let mut part_msg = zmq::Message::new();
		socket.recv(&mut part_msg, 0).unwrap();
		more = part_msg.get_more();
		frame.push(part_msg);
	}

	frame
}

fn build_arbiter_header(server_ident : &str, message_id : u8) -> Vec<zmq::Message> {
	// identity
	// empty (identity delimeter)
	// empty ( spare )
	// empty ( spare )
	// empty ( spare )
	// message ID
	return vec!(zmq::Message::from(server_ident),
		zmq::Message::new(),
		zmq::Message::new(),
		zmq::Message::new(),
		zmq::Message::new(),
		zmq::Message::from(vec!(0, message_id)));
}

fn append_checksum(message : &mut Vec<zmq::Message>) {
	message.push(zmq::Message::new());
}

fn send_register(server_ident : &str, socket : &zmq::Socket) -> zmq::Result<()> {
	let mut register_msg = build_arbiter_header(&server_ident, 0);
	register_msg.push(zmq::Message::new());
	append_checksum(&mut register_msg);
	
	socket.send_multipart(register_msg, 0)
}

fn send_ping(server_ident : &str, socket : &zmq::Socket) -> zmq::Result<()> {
	let mut ping_msg = build_arbiter_header(&server_ident, 2);
	ping_msg.push(zmq::Message::new());
	append_checksum(&mut ping_msg);
	
	socket.send_multipart(ping_msg, 0)
}

fn receive_message(socket : &zmq::Socket) -> zmq::Result<Vec<zmq::Message>> {
	let message = receive_multi(&socket);
	validate_message(message)
}

fn get_message_type(header : &zmq::Message) -> u8 {
	if header.len() >= 2 {
		println!("got msg {}", header[1]);
		header[1]
	} else {
		255
	}
}

fn wait_for_message(message_type : u8, timeout_ms : i64, socket : &zmq::Socket) ->zmq::Result<Vec<zmq::Message>> {
	if socket.poll(zmq::PollEvents::POLLIN, timeout_ms).unwrap() > 0 {
		let result = receive_message(&socket);
		if let Ok(message) = result {
			if get_message_type(&message[5]) == message_type {
				Ok(message)
			} else {
				Err(zmq::Error::EPROTO)
			}
		} else {
			result
		}
	} else {
		Err(zmq::Error::EPROTO)
	}
}

fn register(server_ident : &str, socket : &zmq::Socket) {
	let mut cont = true;
	while cont {
		send_register(&server_ident, &socket).unwrap();
		cont = wait_for_message(3, 1000, &socket).is_err();
	}
	
	println!("Registered");
}

fn main() {
    let ctx = zmq::Context::new();
	let req_socket = ctx.socket(zmq::ROUTER).unwrap();
	let my_logical = "CLIENT";
	let server_ident = "ARBITER";
	
	req_socket.set_identity(my_logical.as_bytes()).unwrap();
	req_socket.connect("tcp://127.0.0.1:5555").unwrap();
	
	register(&server_ident, &req_socket);
	
	loop {
		send_ping(&server_ident, &req_socket).unwrap();
		if wait_for_message(3, 1000, &req_socket).is_ok() {
			println!("got pong!");
			std::thread::sleep(std::time::Duration::from_millis(1000));
		} else {
			println!("bad");
		}
	}
}
