use dora_node_api::{DoraNode, Event, IntoArrow, dora_core::config::DataId};
use serialport::SerialPort;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let (node, mut events) = DoraNode::init_from_env()?;
    let node = Arc::new(Mutex::new(node));
    let node_clone = node.clone();

    // 串口
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 1_000_000;
    let timeout = Duration::from_millis(10);

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open()
        .expect("无法打开串口");

    // 数组内容：id(1), angle(2), circle(1), speed(4)
    let mut encoder_data: Vec<[u8; 8]> = vec![];

    // let mut online_encoder: Vec<u8> = vec![];
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
                // online_encoder.push(scan_id);
                encoder_data.push([scan_id, 0, 0, 0, 0, 0, 0, 0]);
            }
            Ok(_) => {
                println!("ID: {} 无数据", scan_id);
            }
            Err(_e) => {
                println!("ID: {} 离线", scan_id);
            }
        }
    }

    if encoder_data.is_empty() {
        return Err("没有可用的编码器".into());
    }
    let encoder_num = encoder_data.len();

    let display_index: usize = 0;
    let display_index = Arc::new(Mutex::new(display_index));
    let display_index_clone = display_index.clone();

    let mut char_buffer = String::new();
    let encoder_data = Arc::new(Mutex::new(encoder_data));

    let encoder_data_clone = encoder_data.clone();

    // 读取数据进程
    std::thread::spawn(move || {
        loop {
            let mut encoder_data = encoder_data_clone.lock().unwrap();
            for index in 0..encoder_num {
                let current_id = encoder_data[index][0];
                read_speed(&mut port, current_id, &mut encoder_data[index]);
                read_circle(&mut port, current_id, &mut encoder_data[index]);
                read_angle(&mut port, current_id, &mut encoder_data[index]);
            }
        }
    });

    // 持续发送数据线程
    let encoder_data_for_sending = encoder_data.clone();
    std::thread::spawn(move || {
        loop {
            {
                let data: Vec<u8> = encoder_data_for_sending.lock().unwrap()
                    [*display_index_clone.lock().unwrap()]
                .to_vec();
                let metadata = std::collections::BTreeMap::new();
                if let Err(e) = node_clone.lock().unwrap().send_output(
                    DataId::from("encoder_data".to_owned()),
                    metadata,
                    data.into_arrow(),
                ) {
                    eprintln!("发送数据失败: {}", e);
                }
            }
            // 每100毫秒发送一次数据
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, metadata, data } => match id.as_str() {
                "key" => {
                    let message_bytes: Vec<u8> = (&data).try_into()?;
                    let message = String::from_utf8_lossy(&message_bytes);

                    if let Some(ch) = message.chars().next() {
                        match ch {
                            '↑' => {
                                let mut index = display_index.lock().unwrap();
                                if *index < encoder_num - 1 {
                                    *index += 1;
                                } else {
                                    *index = 0;
                                }
                            }
                            '↓' => {
                                let mut index = display_index.lock().unwrap();
                                if *index > 0 {
                                    *index -= 1;
                                } else {
                                    *index = encoder_num - 1;
                                }
                            }
                            '\n' => {
                                match extract_number(&char_buffer) {
                                    Some(num) => {
                                        let encoder_data = encoder_data.lock().unwrap();
                                        if encoder_data.iter().any(|arr| arr[0] == num) {
                                            if let Some(idx) =
                                                encoder_data.iter().position(|arr| arr[0] == num)
                                            {
                                                *display_index.lock().unwrap() = idx;
                                            }
                                        }
                                    }
                                    None => {}
                                }
                                char_buffer.clear();
                            }
                            '⌫' => {
                                char_buffer.pop();
                            }
                            _ => {
                                char_buffer.push(ch);
                            }
                        }
                    }
                    node.lock().unwrap().send_output(
                        DataId::from("char_buffer".to_owned()),
                        metadata.parameters,
                        char_buffer.into_arrow(),
                    )?;
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

fn read_speed(port: &mut Box<dyn SerialPort>, display_id: u8, data: &mut [u8; 8]) {
    let mut send_data = [display_id, 0x03, 0x00, 0x42, 0x00, 0x02, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    data[4..=7].copy_from_slice(&buf[3..=6]);
}

fn read_circle(port: &mut Box<dyn SerialPort>, display_id: u8, data: &mut [u8; 8]) {
    let mut send_data = [display_id, 0x03, 0x00, 0x44, 0x00, 0x01, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    data[3] = buf[4];
}

fn read_angle(port: &mut Box<dyn SerialPort>, display_id: u8, data: &mut [u8; 8]) {
    let mut send_data = [display_id, 0x03, 0x00, 0x41, 0x00, 0x01, 0x00, 0x00];
    let crc = crc16_modbus(&send_data[..6]);
    send_data[6] = (crc & 0xFF) as u8;
    send_data[7] = (crc >> 8) as u8;
    port.write_all(&send_data).expect("发送数据失败");

    let mut buf = [0u8; 256];
    port.read(&mut buf).expect("读取数据失败");
    data[1..=2].copy_from_slice(&buf[3..=4]);
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
