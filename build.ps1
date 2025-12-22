Start-Transcript -Path "c:\Users\Administrator\Downloads\todoapp\todoapp\transcript.log"
$env:Path = "C:\Users\Administrator\Downloads\w64devkit\bin;C:\Users\Administrator\.cargo\bin;$env:Path"
Write-Host "Checking versions..."
rustc --version
cargo --version
Write-Host "Building..."
cargo build --bin todo_cli --verbose
Stop-Transcript
