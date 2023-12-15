// 引入标准库中的文件操作和序列化模块
use std::fs;
use std::io::{self, Write};
use serde::{Serialize, Deserialize};

// 定义一个结构体，用于存储源文件夹和目标文件夹的路径
#[derive(Serialize, Deserialize)]
struct Config {
    src: Vec<String>,
    dst: Vec<String>,
}

// 定义一个函数，用于读取或创建 config.json 文件，并返回一个 Config 结构体的实例
fn get_config() -> Config {
    // 尝试打开 config.json 文件
    match fs::File::open("config.json") {
        // 如果文件存在，就读取文件内容并反序列化为 Config 结构体
        Ok(file) => {
            serde_json::from_reader(file).expect("无法解析 config.json 文件")
        }
        // 如果文件不存在，就创建一个空的 Config 结构体，并序列化为 JSON 字符串，写入新建的 config.json 文件
        Err(_) => {
            let config = Config {
                src: Vec::new(),
                dst: Vec::new(),
            };
            let json = serde_json::to_string_pretty(&config).expect("无法序列化 config 结构体");
            fs::write("config.json", json).expect("无法写入 config.json 文件");
            config
        }
    }
}

// 定义一个函数，用于将源文件夹和目标文件夹的路径添加到 config.json 文件中
fn add_config(src: &str, dst: &str) {
    // 获取当前的 config 结构体
    let mut config = get_config();
    // 将参数转换为 String 类型，并添加到对应的向量中
    config.src.push(src.to_string());
    config.dst.push(dst.to_string());
    // 将更新后的 config 结构体序列化为 JSON 字符串，覆盖写入 config.json 文件
    let json = serde_json::to_string_pretty(&config).expect("无法序列化 config 结构体");
    fs::write("config.json", json).expect("无法写入 config.json 文件");
}

// 定义一个函数，用于将源文件夹中的所有文件复制到目标文件夹中
fn backup_config() {
    // 获取当前的 config 结构体
    let config = get_config();
    // 遍历源文件夹和目标文件夹的路径
    for (src, dst) in config.src.iter().zip(config.dst.iter()) {
        // 读取源文件夹中的所有条目
        let entries = fs::read_dir(src).expect("无法读取源文件夹");
        // 遍历每个条目
        for entry in entries {
            // 如果条目是一个文件，就获取文件的名称和内容
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(name) = entry.file_name().to_str() {
                            if let Ok(content) = fs::read(entry.path()) {
                                // 构造目标文件夹中的文件路径
                                let dst_path = format!("{}/{}", dst, name);
                                // 将文件内容写入目标文件夹中的文件
                                fs::write(dst_path, content).expect("无法写入目标文件夹");
                            }
                        }
                    }
                }
            }
        }
    }
}

// 定义程序的入口函数
fn main() {
    // 获取命令行参数的迭代器
    let mut args = std::env::args();
    // 跳过第一个参数，它是程序的名称
    args.next();
    // 匹配第二个参数，它是程序的命令
    match args.next() {
        // 如果命令是 --add，就获取后面的两个参数，作为源文件夹和目标文件夹的路径，调用 add_config 函数
        Some(cmd) if cmd == "--add" => {
            if let (Some(src), Some(dst)) = (args.next(), args.next()) {
                add_config(&src, &dst);
                println!("成功添加 {} 和 {} 到 config.json 文件", src, dst);
            } else {
                // 如果参数不足，就打印错误信息
                eprintln!("缺少参数：源文件夹和目标文件夹");
            }
        }
        // 如果命令是 --backup 或者没有命令，就调用 backup_config 函数
        Some(cmd) if cmd == "--backup" || cmd == "" => {
            backup_config();
            println!("成功将源文件夹的文件复制到目标文件夹");
        }
        // 如果命令无法识别，就打印错误信息
        Some(cmd) => {
            eprintln!("无法识别的命令：{}", cmd);
        }
        // 如果没有命令，就调用 backup_config 函数
        None => {
            backup_config();
            println!("成功将源文件夹的文件复制到目标文件夹");
        }
    }
}
