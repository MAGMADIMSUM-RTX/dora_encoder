use dora_node_api::{DoraNode, Event};
use linux_embedded_hal::I2cdev;
use serialport::SerialPort;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let (mut _node, mut events) = DoraNode::init_from_env()?;

    // 串口
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 1_000_000;
    let timeout = Duration::from_millis(10);

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open()
        .expect("无法打开串口");

    let mut online_encoder: Vec<u8> = vec![];
    //检测可用编码器数量
    for scan_id in 1..=10 {
        let mut send_data = [scan_id, 0x03, 0x00, 0x41, 0x00, 0x01, 0x00, 0x00];
        let crc = crc16_modbus(&send_data[..6]);
        send_data[6] = (crc & 0xFF) as u8;
        send_data[7] = (crc >> 8) as u8;
        if let Err(e) = port.write_all(&send_data) {
            println!("向 ID: {} 发送数据失败: {}", scan_id, e);
            continue;
        }

        let mut buf = [0u8; 256];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                println!("ID: {} 在线", scan_id);
                online_encoder.push(scan_id);
            }
            Ok(_) => {
                println!("ID: {} 无数据", scan_id);
            }
            Err(_e) => {
                println!("ID: {} 离线", scan_id);
            }
        }
    }

    if online_encoder.is_empty() {
        return Err("没有可用的编码器".into());
    }

    // OLED
    let mut i2c = I2cdev::new("/dev/i2c-1")?;
    oled::init(&mut i2c);

    let display_index: usize = 0;
    let display_index = Arc::new(Mutex::new(display_index));
    let char_buffer = Arc::new(Mutex::new(String::new()));
    let online_encoder = Arc::new(online_encoder);

    let char_buffer_clone = char_buffer.clone();
    let display_index_clone = display_index.clone();
    let online_encoder_clone = online_encoder.clone();

    std::thread::spawn(move || {
        let mut print_buf = oled::Buffer::new(6);
        loop {
            let current_index = *display_index_clone.lock().unwrap();
            let current_buffer = char_buffer_clone.lock().unwrap().clone();

            // 获取当前ID
            let current_id = online_encoder_clone[current_index];
            read_speed(&mut port, current_id, &mut print_buf);
            read_circle(&mut port, current_id, &mut print_buf);
            read_angle(&mut port, current_id, &mut print_buf);
            print_buf.push(format!("ID: {}  ", current_id));
            print_buf.push(format!("$ {}", current_buffer));
            oled::display_buffer(&print_buf, 0, 0, 16, &mut i2c);
            print_buf.push(format!("\n"));
        }
    });

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, metadata:_, data } => match id.as_str() {
                "key" => {
                    let message_bytes: Vec<u8> = (&data).try_into()?;
                    let message = String::from_utf8_lossy(&message_bytes);

                    if let Some(ch) = message.chars().next() {
                        match ch {
                            '↑' => {
                                let mut idx = display_index.lock().unwrap();
                                if *idx < online_encoder.len() - 1 {
                                    *idx += 1;
                                } else {
                                    *idx = 0;
                                }
                            }
                            '↓' => {
                                let mut idx = display_index.lock().unwrap();
                                if *idx > 0 {
                                    *idx -= 1;
                                } else {
                                    *idx = online_encoder.len() - 1;
                                }
                            }
                            '\n' => {
                                match extract_number(&(char_buffer.lock().unwrap())) {
                                    Some(num) => {
                                        if online_encoder.contains(&num) {
                                            if let Some(idx) = online_encoder.iter().position(|&x| x == num) {
                                                *display_index.lock().unwrap() = idx;
                                            }
                                        }
                                    }
                                    None => {
                                    }
                                }
                                char_buffer.lock().unwrap().clear();
                            }
                            '⌫' => {
                                char_buffer.lock().unwrap().pop();
                            }
                            _ => {
                                char_buffer.lock().unwrap().push(ch);
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

fn read_speed(port: &mut Box<dyn SerialPort>, display_id: u8, print_buf: &mut oled::Buffer) {
    let mut send_data = [display_id, 0x03, 0x00, 0x42, 0x00, 0x02, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    print_buf.push(format!(
        "SPRRD: {} RPM\n",
        ((buf[3] as i32) << 24 | (buf[4] as i32) << 16 | (buf[5] as i32) << 8 | (buf[6] as i32))
            as f64
            / 100.0
    ));
    // println!("3收到 {} 字节: {:02X?}", n, &buf[..n]);
}

fn read_circle(port: &mut Box<dyn SerialPort>, display_id: u8, print_buf: &mut oled::Buffer) {
    let mut send_data = [display_id, 0x03, 0x00, 0x44, 0x00, 0x01, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    print_buf.push(format!("CIRCLE: {}\n", buf[4] as i8));
    // println!("2收到 {} 字节: {:02X?}", n, &buf[..n]);
}

fn read_angle(port: &mut Box<dyn SerialPort>, display_id: u8, print_buf: &mut oled::Buffer) {
    let mut send_data = [display_id, 0x03, 0x00, 0x41, 0x00, 0x01, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    print_buf.push(format!(
        "ANGLE: {:.3} '\n",
        (((buf[3] as u16) << 8) | (buf[4] as u16)) as f32 * 360.0 / 4096.0
    ));
}

fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

fn extract_number(s: &str) -> Option<u8> {
    if s.chars().all(|c| c.is_ascii_digit()) {
        s.parse::<u8>().ok()
    } else {
        None
    }
}
