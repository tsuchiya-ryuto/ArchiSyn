use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::commands::load_project;
use archisyn_lib::fs::safe_write::write_files;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let out = std::env::args().nth(2).unwrap();
    let project = load_project(src).unwrap();
    let ws = generate_workspace(&project).unwrap();
    write_files(std::path::Path::new(&out), &ws.files).unwrap();
    println!("generated");
}
