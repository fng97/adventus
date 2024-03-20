use std::path::Path;

// runtime dependencies workaround: https://github.com/shuttle-hq/shuttle/issues/703#issuecomment-1515606621

fn main() {
    // only for shuttle environment
    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        let install_path = Path::new("/usr/local/bin/yt-dlp");

        // download yt-dlp using curl
        if !std::process::Command::new("curl")
            .arg("-L")
            .arg("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp")
            .arg("-o")
            .arg(install_path)
            .status()
            .expect("Failed to execute curl command")
            .success()
        {
            panic!("Failed to download yt-dlp");
        }

        // make yt-dlp executable
        if !std::process::Command::new("chmod")
            .arg("a+rx")
            .arg(install_path)
            .status()
            .expect("Failed to execute chmod command")
            .success()
        {
            panic!("Failed to make yt-dlp executable");
        }

        println!("yt-dlp has been installed successfully!");
    }
}
