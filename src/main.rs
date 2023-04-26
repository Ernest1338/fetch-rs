use std::collections::HashMap;

const HEADER: &str = "\x1b[33m";
const TEXT: &str = "\x1b[97m";
const RESET: &str = "\x1b[00m";
const LOGO: [&str; 10] = [
    "             ",
    "____________ ",
    "| ___ \\  ___|",
    "| |_/ / |_   ",
    "|    /|  _|  ",
    "| |\\ \\| |    ",
    "\\_| \\_\\_|    ",
    "             ",
    "             ",
    " \x1b[41m  \x1b[42m  \x1b[43m  \x1b[44m  \x1b[45m  \x1b[46m  \x1b[00m",
];
static mut CURRENT_LOGO_LINE: usize = 0;

fn display_logo_line() {
    // TODO: remove this unsafe block
    unsafe {
        if CURRENT_LOGO_LINE > LOGO.len() - 1 {
            print!("{}", " ".repeat(LOGO[0].len() + 1));
        } else {
            print!("{} ", LOGO[CURRENT_LOGO_LINE]);
            CURRENT_LOGO_LINE += 1;
        }
    }
}

fn cmd_out(command: &str, user_args: &[&str]) -> Option<String> {
    let args = std::env::args().collect::<Vec<String>>(); // TODO: cache this call?
    if args.len() > 1 && args[1] == "--fast" {
        return None;
    }
    let command = std::process::Command::new(command).args(user_args).output();

    if let Ok(ok) = command {
        Some(String::from_utf8(ok.stdout).expect("ERROR converting to utf-8"))
    } else {
        None
    }
}

fn display(text1: &str, text2: &str) {
    println!("{HEADER}{text1}{TEXT}{text2}");
}

fn display_cpu_model() {
    let file = std::fs::read_to_string("/proc/cpuinfo").unwrap();
    for line in file.lines() {
        if line.contains("model name") {
            display_logo_line();
            display("CPU Model:", line.split(':').collect::<Vec<&str>>()[1]);
            return;
        }
    }
}

fn helper_mem_line(line: &str) -> usize {
    line.split(':').collect::<Vec<&str>>()[1]
        .split_whitespace()
        .collect::<Vec<&str>>()[0]
        .parse::<usize>()
        .unwrap()
}

fn display_mem_info() {
    let file = std::fs::read_to_string("/proc/meminfo").unwrap();
    let (mut mem_total, mut mem_used, mut swap_total, mut swap_used) = (0, 0, 0, 0);
    for line in file.lines() {
        if line.contains("MemTotal") {
            mem_total = helper_mem_line(line);
        }
        if line.contains("MemAvailable") {
            mem_used = mem_total - helper_mem_line(line);
        }
        if line.contains("SwapTotal") {
            swap_total = helper_mem_line(line)
        };
        if line.contains("SwapFree") {
            swap_used = swap_total - helper_mem_line(line);
            // assuming SwapFree is the most bottom line out of these 4, we can break here
            break;
        }
    }
    display_logo_line();
    display(
        "Swap: ",
        &format!(
            "{:.2} MB / {:.2} MB",
            swap_used as f32 / 1000.0,
            swap_total as f32 / 1000.0
        ),
    );
    display_logo_line();
    display(
        "Memory: ",
        &format!(
            "{:.2} MB / {:.2} MB",
            mem_used as f32 / 1000.0,
            mem_total as f32 / 1000.0
        ),
    );
}

fn display_load_avg() {
    let file = std::fs::read_to_string("/proc/loadavg").unwrap();
    let split: Vec<&str> = file.split(' ').collect();
    display_logo_line();
    display(
        "Load average: ",
        &format!("{} {} {}", split[0], split[1], split[2]),
    );
}

fn display_session_type(env_vars: &HashMap<String, String>) {
    display_logo_line();
    display(
        "Session type: ",
        if env_vars.get("WAYLAND_DISPLAY").is_some() {
            "Wayland"
        } else {
            "X11"
        },
    );
}

fn display_package_count() {
    let mut package_count = 0;
    // try arch
    if std::path::Path::new("/var/lib/pacman/local/").exists() {
        for _ in std::fs::read_dir("/var/lib/pacman/local/").unwrap() {
            package_count += 1;
        }
    } else {
        // try debian
        // TODO: avoid calling dpkg-query
        let debian_out = match cmd_out("dpkg-query", &["-l"]) {
            Some(ok) => ok,
            None => return, // --fast
        };
        let debian_out_len = debian_out.lines().count();
        if debian_out_len > 10 {
            package_count = debian_out_len;
        }
    }
    display_logo_line();
    display("Installed package count: ", &format!("{package_count}"));
}

fn display_process_count() {
    display_logo_line();
    let mut process_count = 0;
    for entry in std::fs::read_dir("/proc").unwrap() {
        if entry
            .unwrap()
            .file_name()
            .to_str()
            .unwrap()
            .chars()
            .all(|c| c.is_ascii_digit())
        {
            process_count += 1;
        }
    }
    display("Process count: ", &format!("{process_count}",));
}

fn display_list_of_data(title: &str, data: &[String]) {
    if !data.is_empty() {
        display_logo_line();
        display(title, "");
        for thing in data {
            display_logo_line();
            display("   ", thing);
        }
    }
}

fn display_lspci_data() {
    let mut gpus: Vec<String> = Vec::new();
    let mut network_adapters: Vec<String> = Vec::new();
    let mut audio_devices: Vec<String> = Vec::new();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() > 1 && args[1] == "--fast" {
        // this fast version only works on some devices (not sure why)
        for entry in std::fs::read_dir("/sys/bus/pci/devices/").unwrap() {
            let entry_path = entry.unwrap().path();
            // the problem is, there is no label file on some devices
            let contents = std::fs::read_to_string(entry_path.join("label"));
            if let Ok(content) = contents {
                let content = content.trim();
                let dev_type = std::fs::read_to_string(entry_path.join("class")).unwrap();
                match &dev_type[2..4] {
                    // data from /usr/share/hwdata/pci.ids
                    "03" => gpus.push(content.to_string()), // GPU
                    "02" | "0d" => network_adapters.push(content.to_string()), // Networking
                    _ => (),
                }
            }
        }
    } else {
        let lspci_data = cmd_out("lspci", &[]).expect("ERROR getting data from lspci");
        for line in lspci_data.lines() {
            if line.contains("VGA compatible") {
                gpus.push(line.split(':').collect::<Vec<&str>>()[2].to_string());
            }
            if line.contains("Ethernet controller") {
                network_adapters.push(line.split(':').collect::<Vec<&str>>()[2].to_string());
            }
            if line.contains("Audio device") {
                audio_devices.push(line.split(':').collect::<Vec<&str>>()[2].to_string());
            }
        }
    }

    display_list_of_data("Display controller(s) - GPU(s):", &gpus);
    display_list_of_data("Network adapter(s):", &network_adapters);
    display_list_of_data("Audio device(s):", &audio_devices);
}

fn display_env_var(env_vars: &HashMap<String, String>, title: &str, var: &str) {
    let var = env_vars.get(var);
    if let Some(var) = var {
        display_logo_line();
        display(title, var);
    }
}

fn display_language(env_vars: &HashMap<String, String>) {
    display_logo_line();
    display(
        "Language: ",
        env_vars
            .get("LANG")
            .unwrap()
            .split('.')
            .collect::<Vec<&str>>()[0],
    );
}

fn display_user_hostname(env_vars: &HashMap<String, String>) {
    let user = env_vars.get("USER").unwrap();
    let hostname = std::fs::read_to_string("/etc/hostname").unwrap();
    display_logo_line();
    print!("{HEADER}{user}@{hostname}{TEXT}");
    display_logo_line();
    println!("{}", "=".repeat(user.len() + hostname.len()));
}

fn display_kernel_version() {
    display_logo_line();
    display(
        "Kernel version: ",
        std::fs::read_to_string("/proc/version")
            .unwrap()
            .split(' ')
            .collect::<Vec<&str>>()[2],
    );
}

fn display_uptime() {
    display_logo_line();
    display(
        "Uptime: ",
        &format!(
            "{:.2} h", // TODO: use a more human readable output format
            std::fs::read_to_string("/proc/uptime")
                .unwrap()
                .split(' ')
                .collect::<Vec<&str>>()[0]
                .parse::<f32>()
                .unwrap()
                / 3600.0,
        ),
    );
}

fn display_os_name() {
    display_logo_line();
    display(
        "Operating System: ",
        std::fs::read_to_string("/etc/os-release")
            .unwrap()
            .lines()
            .collect::<Vec<&str>>()[0]
            .split('\"')
            .collect::<Vec<&str>>()[1],
    );
}

fn main() {
    // TODO: display_disk_usage
    // TODO: allow customizing which info to display
    let env_vars: HashMap<String, String> = std::env::vars().collect();
    display_user_hostname(&env_vars);
    display_os_name();
    display_kernel_version();
    display_cpu_model();
    display_lspci_data();
    display_session_type(&env_vars);
    display_load_avg();
    display_package_count();
    display_process_count();
    display_env_var(&env_vars, "Shell: ", "SHELL");
    display_env_var(&env_vars, "Editor: ", "EDITOR");
    display_language(&env_vars);
    // TODO: i'm not sure if this is THE way of extracting this information
    display_env_var(&env_vars, "Environment: ", "XDG_CURRENT_DESKTOP");
    display_env_var(&env_vars, "Terminal: ", "TERM");
    display_uptime();
    display_mem_info();
    print!("{RESET}");
}
