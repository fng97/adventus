use std::path::Path;

fn main() {
    // Only proceed if the HOSTNAME environment variable indicates we are in a "shuttle" environment
    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        // Define the installation path for yt-dlp within a Docker container
        let install_path = Path::new("/usr/local/bin/yt-dlp");

        // Download yt-dlp using curl
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

        // Make yt-dlp executable
        if !std::process::Command::new("chmod")
            .arg("a+rx")
            .arg(install_path)
            .status()
            .expect("Failed to execute chmod command")
            .success()
        {
            panic!("Failed to make yt-dlp executable");
        }

        println!("yt-dlp has been installed successfully.");
    }
}
