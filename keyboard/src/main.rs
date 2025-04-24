use dora_node_api::{DoraNode, IntoArrow, dora_core::config::DataId};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::thread;
use std::time::Duration;

#[repr(C)]
struct InputEvent {
    tv_sec: usize,
    tv_usec: usize,
    type_: u16,
    code: u16,
    value: i32,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("尝试打开 /dev/input/event5");
    let mut file = File::open("/dev/input/event5")?;
    println!("成功打开");

    let mut buffer = [0u8; std::mem::size_of::<InputEvent>()];

    let (mut node, mut _events) = DoraNode::init_from_env()?;
    loop {
        // 读取输入事件
        match file.read_exact(&mut buffer) {
            Ok(_) => {
                // 将字节缓冲区转换为InputEvent结构
                let event = unsafe { std::ptr::read(buffer.as_ptr() as *const InputEvent) };

                // 处理不同类型的事件
                match event.type_ {
                    1 => {
                        // 读按键
                        let key_str: &str = match event.code {
                            2 => "1" ,
                            3 => "2" ,
                            4 => "3" ,
                            5 => "4",
                            6 => "5",
                            7 => "6",
                            8 => "7",
                            9 => "8",
                            10 => "9",
                            11 => "0" ,
                            12 => "-" ,
                            13 => "=" ,
                            14 => "⌫" , // BACKSPACE
                            15 => "\t" , // TAB
                            16 => "q" ,
                            17 => "w" ,
                            18 => "e",
                            19 => "r",
                            20 => "t",
                            21 => "y",
                            22 => "u",
                            23 => "i",
                            24 => "o",
                            25 => "p" ,
                            26 => "[" ,
                            27 => "]" ,
                            28 => "\n" , // ENTER
                            29 => "L_CTRL" , // LEFT CTRL
                            30 => "a" ,
                            31 => "s" ,
                            32 => "d",
                            33 => "f",
                            34 => "g",
                            35 => "h",
                            36 => "j",
                            37 => "k",
                            38 => "l" ,
                            39 => ";" ,
                            40 => &('"'.to_string()) ,
                            41 => "`" ,
                            42 => "L_SHIFT" , // LEFT SHIFT
                            43 => "\\" ,
                            44 => "z" ,
                            45 => "x",
                            46 => "c",
                            47 => "v",
                            48 => "b",
                            49 => "n",
                            50 => "m",
                            51 => "," ,
                            52 => "." ,
                            53 => "/" ,
                            54 => "R_SHIFT" , // RIGHT SHIFT
                            56 => "L_ALT" , // LEFT ALT
                            57 => " " , // SPACE
                            58 => "CAPS_LOCK" , // CAPS LOCK
                            59 => "F1" ,
                            60 => "F2" ,
                            61 => "F3" ,
                            62 => "F4",
                            63 => "F5",
                            64 => "F6",
                            65 => "F7",
                            66 => "F8",
                            67 => "F9",
                            68 => "F10",
                            69 => "F11",
                            70 => "F12",
                            71 => "NUM", // NUMLOCK
                            97 => "^", // RIGHT CTRL
                            100 => "⎇", // RIGHT ALT
                            102 => "⇱", // HOME
                            103 => "↑", // UP
                            104 => "⇞", // PAGE UP
                            105 => "←", // LEFT
                            106 => "→", // RIGHT
                            107 => "⇲", // END
                            108 => "↓", // DOWN
                            109 => "⇟", // PAGE DOWN
                            110 => "⎀", // INSERT
                            111 => "⌦", // DELETE
                            119 => "⏸", // PAUSE
                        _ => "?",
                        };
                        // println!("按键: {}, value: {}", key_str, event.value);
                        if event.value == 1 {
                            let bytes: Vec<u8> = key_str.as_bytes().to_vec();
                            let metadata = std::collections::BTreeMap::new();
                            node.send_output(
                                DataId::from("key".to_owned()),
                                metadata,
                                bytes.into_arrow(),
                            )?;
                        }
                    }
                    _ => {}
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                println!("设备已断开连接");
                break;
            }
            Err(e) => {
                eprintln!("读取错误: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    Ok(())
}
