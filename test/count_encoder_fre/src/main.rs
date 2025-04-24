use serialport::SerialPort;
use std::time::{Duration, Instant};

fn main() {
    // 串口
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 1_000_000;
    let timeout = Duration::from_millis(100);

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open()
        .expect("无法打开串口");

    // // 用于频率计算的变量
    // let mut last_time = Instant::now();
    // let mut message_count = 0;
    // let mut total_time = Duration::new(0, 0);
    // let update_interval = Duration::from_secs(1); // 每秒更新一次频率显示
    // let mut next_update = Instant::now() + update_interval;

    loop {
        let send_time = Instant::now();
        let mut send_data = [1, 0x03, 0x00, 0x42, 0x00, 0x02, 0x00, 0x00];
        let crc = crc16_modbus(&send_data[..6]);
        send_data[6] = (crc & 0xFF) as u8;
        send_data[7] = (crc >> 8) as u8;
        port.write_all(&send_data).expect("发送数据失败");

        let mut buf = [0u8; 10];
        port.read(&mut buf).expect("读取数据失败");
        let recv_time = Instant::now();

        // 计算发送和接收的间隔
        let send_recv_interval = recv_time.duration_since(send_time);
        println!(
            "本次发送到接收的间隔: {:.3} ms",
            send_recv_interval.as_secs_f64() * 1000.0
        );

        // // 计算接受消息的频率
        // let now = recv_time;
        // let time_diff = now.duration_since(last_time);
        // total_time += time_diff;
        // message_count += 1;
        // last_time = now;

        // // 定期更新和显示频率
        // if now >= next_update {
        //     let avg_frequency = message_count as f64 / total_time.as_secs_f64();
        //     println!(
        //         "接收消息频率: {:.2} Hz (每秒接收 {:.2} 条消息)",
        //         avg_frequency, avg_frequency
        //     );

        //     // 重置计数器以避免溢出，并保持平滑的平均值
        //     message_count = 0;
        //     total_time = Duration::new(0, 0);
        //     next_update = now + update_interval;
        // }
    }
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
