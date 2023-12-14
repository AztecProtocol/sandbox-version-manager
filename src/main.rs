/// TODO:
/// - check docker is available
/// - check if docker is running
use clap::{Args, Parser};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use tar::Archive;

const AZTEC_DIR: &str = "~/.aztec";
const AZTEC_REPO: &str = "aztecprotocol/aztec-sandbox";

const COMPOSE_TEXT: &str = r#"
version: '3'
services:
  ethereum:
    image: ghcr.io/foundry-rs/foundry:v1.0.0
    command: '"anvil --silent -p 8545 --host 0.0.0.0 --chain-id 31337"'
    ports:
      - '${SANDBOX_ANVIL_PORT:-8545}:8545'

  aztec:
    image: 'aztecprotocol/aztec-sandbox:${SANDBOX_VERSION:-latest}'
    ports:
      - '${SANDBOX_RPC_PORT:-8080}:8080'
    environment:
      DEBUG: # Loaded from the user shell if explicitly set
      HOST_WORKDIR: '${PWD}' # Loaded from the user shell to show log files absolute path in host
      ETHEREUM_HOST: http://ethereum:8545
      CHAIN_ID: 31337
      ARCHIVER_POLLING_INTERVAL_MS: 50
      P2P_BLOCK_CHECK_INTERVAL_MS: 50
      SEQ_TX_POLLING_INTERVAL_MS: 50
      WS_BLOCK_CHECK_INTERVAL_MS: 50
      RPC_SERVER_BLOCK_POLLING_INTERVAL_MS: 50
      ARCHIVER_VIEM_POLLING_INTERVAL_MS: 500
    volumes:
      - ./log:/usr/src/yarn-project/aztec-sandbox/log:rw
"#;

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Aztec Labs")]
#[command(author, version, about, long_about = None)]
enum AztecVersionManagerCommand {
    #[clap(about = "Install a specific aztec-sandbox image version")]
    Install(Install),

    #[clap(about = "Set the aztec-sandbox version to use")]
    Use(Use),

    #[clap(about = "Run the sandbox")]
    Run,

    #[clap(about = "Update the aztec sandbox version manager")]
    Update,
}

#[derive(Args, Debug)]
struct Install {
    tag: String,
}

#[derive(Args, Debug)]
struct Use {
    version: String,
}

fn main() {
    let cmd: AztecVersionManagerCommand = AztecVersionManagerCommand::parse();

    match cmd {
        AztecVersionManagerCommand::Install(cmd) => install(&cmd.tag),
        AztecVersionManagerCommand::Use(cmd) => use_version(&cmd.version),
        AztecVersionManagerCommand::Run => run(),
        AztecVersionManagerCommand::Update => update(),
    }
}

fn install(tag: &str) {
    let docker_image = format!("{}:{}", AZTEC_REPO, tag);
    let status = Command::new("docker")
        .arg("pull")
        .arg(docker_image)
        .status()
        .expect("Failed to execute command");

    if status.success() {
        println!("Image {} installed successfully.", tag);
    } else {
        eprintln!("Failed to install image {}.", tag);
    }
}

fn use_version(version: &str) {
    // TODO: deprecated home dir command -> look in noir
    let path = PathBuf::from(AZTEC_DIR.replace('~', &dirs::home_dir().unwrap().to_string_lossy()))
        .join("version");
    fs::write(path, version).expect("Failed to write to version file.");
    println!("Set version to: {}", version);
}

fn write_compose_text() {
    let path = PathBuf::from(AZTEC_DIR.replace('~', &dirs::home_dir().unwrap().to_string_lossy()))
        .join("run");
    if !path.exists() {
        fs::write(path, COMPOSE_TEXT).expect("Failed to write to compose file.");
    }
}

fn run() {
    let base = PathBuf::from(AZTEC_DIR.replace('~', &dirs::home_dir().unwrap().to_string_lossy()));
    let compose_path = &base.join("run");
    // TODO:cleanup
    write_compose_text();
    let version_path = &base.join("version");

    if !compose_path.exists() {
        eprintln!("No docker-compose file found in ~/.aztec/run");
        return;
    }
    if !version_path.exists() {
        eprintln!("No version file found in ~/.aztec/run");
        return;
    }

    // set env vars
    let version = fs::read_to_string(version_path).expect("Failed to read version file.");
    // write the version to SANDBOX_VERSION
    std::env::set_var("SANDBOX_VERSION", version);

    let status = std::process::Command::new("docker-compose")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .arg("-f")
        .arg(compose_path.to_string_lossy().to_string())
        .arg("up")
        .status()
        .expect("Failed to execute docker-compose.");

    if !status.success() {
        eprintln!("Command did not execute successfully.");
    }
}

fn update() {
    println!("Updating to the latest version...");
    println!("Downloading latest version...");
    let url_result = get_tar_url();
    if url_result.is_err() {
        eprintln!("Could not get latest version.");
        return;
    }
    let url = url_result.unwrap();

    println!("Downloading from: {}", &url);

    // Get the arch
    let response = reqwest::blocking::get(&url).expect("Failed to get latest version.");
    let content = response.bytes().expect("Could not process bytes");

    let reader = Cursor::new(content);

    let tar = GzDecoder::new(reader);
    let mut archive = Archive::new(tar);

    let aztec_dir =
        PathBuf::from(AZTEC_DIR.replace('~', &dirs::home_dir().unwrap().to_string_lossy()));
    archive
        .unpack(&aztec_dir.join("bin"))
        .expect("Could not unpack archive");

    let binary_path = PathBuf::from(&aztec_dir).join("bin/aztec-sandbox");
    let _ = Command::new("chmod").arg("+x").arg(&binary_path).status();

    println!("Installation complete.");
}

fn get_tar_url() -> Result<String, String> {
    let architecture = Command::new("uname")
        .arg("-m")
        .output()
        .expect("Failed to execute command")
        .stdout;

    // Convert stdout bytes to String and trim newline
    let mut arch_string = String::from_utf8(architecture)
        .expect("Not UTF8")
        .trim()
        .to_string();

    let plat = Command::new("uname")
        .arg("-s")
        .output()
        .expect("Failed to execute command")
        .stdout;

    let plat_s = String::from_utf8(plat)
        .expect("Not UTF8")
        .trim()
        .to_string();

    let plat_string = match plat_s.as_str() {
        "Darwin" => "apple-darwin",
        "Linux" => "unknown-linux-gnu",
        _ => {
            eprintln!("unsupported platform: {}", plat_s);
            return Err("unsupported platform".into());
        }
    };

    match arch_string.as_str() {
        "arm64" => arch_string = "aarch64".to_string(),
        "x86_64" | "aarch64" => {}
        _ => {
            eprintln!("unsupported architecture: {}-PLATFORM", arch_string);
            return Err("unsupported arch".into());
        }
    }

    let repo = "AztecProtocol/sandbox-version-manager";
    let tag = "nightly";
    let release_url = format!("https://github.com/{}/releases/download/{}", repo, tag);
    let bin_tarball_url = format!(
        "{}/aztec-sandbox-{}-{}.tar.gz",
        release_url, arch_string, plat_string
    );

    Ok(bin_tarball_url)
}
