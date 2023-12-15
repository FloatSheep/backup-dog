// 引入标准库中的文件系统模块和命令行参数模块
use std::fs;
use std::env;

// 定义一个结构体，用于存储源文件夹和目标文件夹的路径
#[derive(Debug)]
struct FolderPair {
    src: String,
    dst: String,
}

// 定义一个函数，用于读取或创建 config.json 文件，并返回一个包含文件夹对的向量
fn read_or_create_config() -> Vec<FolderPair> {
    // 定义 config.json 文件的路径
    let config_path = "config.json";
    // 定义一个空的向量，用于存储文件夹对
    let mut folder_pairs = Vec::new();
    // 尝试打开 config.json 文件
    match fs::File::open(config_path) {
        // 如果文件存在，就读取文件内容，并解析成 JSON 对象
        Ok(file) => {
            let json: serde_json::Value = serde_json::from_reader(file).unwrap();
            // 遍历 JSON 对象中的数组，将每个元素转换成 FolderPair 结构体，并添加到向量中
            for pair in json.as_array().unwrap() {
                let src = pair["src"].as_str().unwrap().to_string();
                let dst = pair["dst"].as_str().unwrap().to_string();
                folder_pairs.push(FolderPair { src, dst });
            }
        }
        // 如果文件不存在，就创建一个空的文件，并初始化一个空的 JSON 数组
        Err(_) => {
            let file = fs::File::create(config_path).unwrap();
            serde_json::to_writer(file, &serde_json::json!([])).unwrap();
        }
    }
    // 返回文件夹对向量
    folder_pairs
}

// 定义一个函数，用于将源文件夹和目标文件夹添加到 config.json 文件中
fn add_folder_pair(src: &str, dst: &str) {
    // 定义 config.json 文件的路径
    let config_path = "config.json";
    // 读取 config.json 文件的内容，并解析成 JSON 对象
    let file = fs::File::open(config_path).unwrap();
    let mut json: serde_json::Value = serde_json::from_reader(file).unwrap();
    // 创建一个新的 JSON 对象，表示文件夹对
    let new_pair = serde_json::json!({
        "src": src,
        "dst": dst
    });
    // 将新的 JSON 对象添加到原来的 JSON 数组中
    json.as_array_mut().unwrap().push(new_pair);
    // 重新写入 config.json 文件
    let file = fs::File::create(config_path).unwrap();
    serde_json::to_writer(file, &json).unwrap();
    // 打印提示信息
    println!("添加成功：{} -> {}", src, dst);
}

// 定义一个函数，用于将源文件夹中的所有文件复制到目标文件夹中
fn copy_folder(src: &str, dst: &str) {
    // 遍历源文件夹中的所有条目
    for entry in fs::read_dir(src).unwrap() {
        // 获取条目的元数据
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        // 获取条目的路径和文件名
        let src_path = entry.path();
        let file_name = src_path.file_name().unwrap();
        // 根据目标文件夹和文件名构造目标路径
        let mut dst_path = fs::canonicalize(dst).unwrap();
        dst_path.push(file_name);
        // 如果条目是文件，就复制到目标路径
        if metadata.is_file() {
            fs::copy(&src_path, &dst_path).unwrap();
        }
        // 如果条目是文件夹，就递归地复制到目标路径
        if metadata.is_dir() {
            fs::create_dir_all(&dst_path).unwrap();
            copy_folder(
                src_path.to_str().unwrap(),
                dst_path.to_str().unwrap(),
            );
        }
    }
}

// 定义一个函数，用于将所有文件夹对中的源文件夹复制到目标文件夹中
fn backup_all(folder_pairs: &[FolderPair]) {
    // 遍历文件夹对向量
    for pair in folder_pairs {
        // 获取源文件夹和目标文件夹的路径
        let src = &pair.src;
        let dst = &pair.dst;
        // 调用复制函数
        copy_folder(src, dst);
        // 打印提示信息
        println!("备份成功：{} -> {}", src, dst);
    }
}

// 定义一个函数，用于将程序注册到 Windows 服务中
fn register_service() {
    // 获取当前程序的路径
    let exe_path = env::current_exe().unwrap();
    // 获取当前程序的进程 ID
    let pid = std::process::id();
    // 构造注册服务的命令
    let command = format!(
        "sc create BackupDog binPath= \"{}\" start= auto obj= LocalSystem",
        exe_path.to_str().unwrap()
    );
    // 调用系统命令
    std::process::Command::new("cmd")
        .args(&["/C", &command])
        .output()
        .unwrap();
    // 打印提示信息
    println!("注册成功，服务名为 BackupDog，进程 ID 为 {}", pid);
}

// 定义一个函数，用于将程序从 Windows 服务中反注册
fn unregister_service() {
    // 调用系统命令
    std::process::Command::new("cmd")
        .args(&["/C", "sc delete BackupDog"])
        .output()
        .unwrap();
    // 打印提示信息
    println!("反注册成功，服务名为 BackupDog");
}

// 主函数
fn main() {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    // 判断命令行参数的个数
    match args.len() {
        // 如果只有一个参数，表示没有指定任何操作，就执行备份操作
        1 => {
            // 读取 config.json 文件，并获取文件夹对向量
            let folder_pairs = read_or_create_config();
            // 调用备份函数
            backup_all(&folder_pairs);
        }
        // 如果有两个参数，表示指定了一个操作，根据操作的不同，执行不同的函数
        2 => {
            // 获取第二个参数，作为操作
            let operation = &args[1];
            // 判断操作的内容
            match operation.as_str() {
                // 如果是 --register，就调用注册服务函数
                "--register" => register_service(),
                // 如果是 --unregister，就调用反注册服务函数
                "--unregister" => unregister_service(),
                // 如果是其他内容，就打印错误信息
                _ => println!("无效的操作：{}", operation),
            }
        }
        // 如果有四个参数，表示指定了一个操作和两个文件夹路径，根据操作的不同，执行不同的函数
        4 => {
            // 获取第二个参数，作为操作
            let operation = &args[1];
            // 获取第三个参数，作为源文件夹路径
            let src = &args[2];
            // 获取第四个参数，作为目标文件夹路径
            let dst = &args[3];
            // 判断操作的内容
            match operation.as_str() {
                // 如果是 --add，就调用添加文件夹对函数
                "--add" => add_folder_pair(src, dst),
                // 如果是其他内容，就打印错误信息
                _ => println!("无效的操作：{}", operation),
            }
        }
        // 如果参数个数不是 1、2 或 4，就打印错误信息
        _ => println!("无效的参数个数：{}", args.len()),
    }
}
