use dora_node_api::{DoraNode, Event};
use linux_embedded_hal::I2cdev;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let (_node, mut events) = DoraNode::init_from_env()?;

    // OLED
    let mut i2c = I2cdev::new("/dev/i2c-1")?;
    oled::init(&mut i2c);
    let mut print_buf = oled::Buffer::new(6);
    let mut current_buffer = String::new();
    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata: _,
                data,
            } => match id.as_str() {
                "encoder_data" => {
                    let data: Vec<u8> = (&data).try_into()?;
                    print_buf.push(format!(
                        "SPRRD: {:.2} RPM\n",
                        ((data[4] as i32) << 24
                            | (data[5] as i32) << 16
                            | (data[6] as i32) << 8
                            | (data[7] as i32)) as f64
                            / 100.0
                    ));
                    print_buf.push(format!("CIRCLE: {}\n", data[3] as i8));
                    print_buf.push(format!(
                        "ANGLE: {:.2}'\n",
                        (((data[1] as u16) << 8 | (data[2] as u16)) as f32) * 360.0 / 4096.0
                    ));
                    print_buf.push(format!("ID: {}  ", data[0]));
                    print_buf.push(format!("$ {}", current_buffer));
                    oled::display_buffer(&print_buf, 0, 0, 16, &mut i2c);
                    print_buf.push(format!("\n"));
                }
                "char_buffer" => {
                    let data: String = (&data).try_into()?;
                    current_buffer = data.clone();
                }
                other => eprintln!("Received input `{other}`"),
            },
            _ => {}
        }
    }

    Ok(())
}
