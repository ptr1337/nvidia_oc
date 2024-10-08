use clap::{arg, Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use nvml_wrapper::Nvml;
use nvml_wrapper_sys::bindings::{nvmlDevice_t, NvmlLib};
use std::io;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Sets GPU parameters like frequency offset and power limit
    Set {
        /// GPU index
        #[arg(short, long)]
        index: u32,

        #[command(flatten)]
        sets: Sets,
    },
    /// Generate shell completion script
    Completion {
        /// The shell to generate the script for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Args, Debug)]
#[group(required = true, multiple = true)]
struct Sets {
    /// GPU frequency offset
    #[arg(short, long)]
    freq_offset: Option<i32>,
    /// GPU memory frequency offset
    #[arg(short, long)]
    mem_offset: Option<i32>,
    /// GPU power limit in milliwatts
    #[arg(short, long)]
    power_limit: Option<u32>,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Set {
            index,
            sets:
                Sets {
                    freq_offset,
                    mem_offset,
                    power_limit,
                },
        } => {
            sudo2::escalate_if_needed()
                .or_else(|_| sudo2::doas())
                .or_else(|_| sudo2::pkexec())
                .expect("Failed to escalate privileges");

            let nvml = Nvml::init().expect("Failed to initialize NVML");

            let device = nvml.device_by_index(*index).expect("Failed to get GPU");

            unsafe {
                let raw_device_handle: nvmlDevice_t = device.handle();
                let nvml_lib =
                    NvmlLib::new("libnvidia-ml.so").expect("Failed to load NVML library");

                if let Some(offset) = freq_offset {
                    set_gpu_frequency_offset(&nvml_lib, raw_device_handle, *offset)
                        .expect("Failed to set GPU frequency offset");
                }

                if let Some(offset) = mem_offset {
                    set_gpu_memory_frequency_offset(&nvml_lib, raw_device_handle, *offset)
                        .expect("Failed to set GPU memory frequency offset");
                }

                if let Some(limit) = power_limit {
                    set_gpu_power_limit(&nvml_lib, raw_device_handle, *limit)
                        .expect("Failed to set GPU power limit");
                }
            }
            println!("Successfully set GPU parameters.");
        }
        Commands::Completion { shell } => {
            generate_completion_script(*shell);
        }
    }
}

fn set_gpu_frequency_offset(
    nvml_lib: &NvmlLib,
    handle: nvmlDevice_t,
    offset: i32,
) -> Result<(), String> {
    let result = unsafe { nvml_lib.nvmlDeviceSetGpcClkVfOffset(handle, offset) };
    if result != 0 {
        Err(format!("Error code: {}", result))
    } else {
        Ok(())
    }
}

fn set_gpu_memory_frequency_offset(
    nvml_lib: &NvmlLib,
    handle: nvmlDevice_t,
    offset: i32,
) -> Result<(), String> {
    let result = unsafe { nvml_lib.nvmlDeviceSetMemClkVfOffset(handle, offset) };
    if result != 0 {
        Err(format!("Error code: {}", result))
    } else {
        Ok(())
    }
}

fn set_gpu_power_limit(nvml_lib: &NvmlLib, handle: nvmlDevice_t, limit: u32) -> Result<(), String> {
    let result = unsafe { nvml_lib.nvmlDeviceSetPowerManagementLimit(handle, limit) };
    if result != 0 {
        Err(format!("Error code: {}", result))
    } else {
        Ok(())
    }
}

fn generate_completion_script<G: Generator>(gen: G) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(gen, &mut cmd, name, &mut io::stdout());
}
