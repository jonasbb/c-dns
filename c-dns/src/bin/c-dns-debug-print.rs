use c_dns::serialization::File;
use misc_utils::fs;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    // Read all files passed on the command line
    let mut args = env::args_os().peekable();
    // Skip program name
    args.next();

    if args.len() == 0 {
        print_help();
        return Ok(());
    }

    let mut dump_serialized = false;
    match args.peek() {
        Some(x) | Some(x) if x == OsStr::new("-h") || x == OsStr::new("--help") => {
            print_help();
            return Ok(());
        }
        Some(x) if x == OsStr::new("--dump-serialized") => {
            dump_serialized = true;
            args.next();
        }
        _ => {}
    }

    for file in args {
        let file = Path::new(&file);
        let buffer = fs::read(file)?;
        match serde_path_to_error::deserialize::<_, File>(&mut serde_cbor::Deserializer::from_reader(
            buffer.as_slice(),
        )) {
            Ok(cdns) => {
                println!(
                    "====================\nFile: {}\n====================\n",
                    file.display(),
                );
                println!("{:#?}", cdns);

                if dump_serialized {
                    let mut reserialized = Vec::new();
                    serde_cbor::to_writer(&mut reserialized, &cdns).unwrap();
                    let newfile = file.with_extension("new.cdns");
                    std::fs::write(newfile, reserialized).unwrap();
                }
            }
            Err(error) => eprintln!(
                "====================\nFailed to deserialize: {}\n====================\n{}\n",
                error.path(),
                error.inner()
            ),
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        r#"Test if a C-DNS file can be parsed.
Print the content of the file in human readable form.

Arguments:
--help, -h: Print this help message
--dump-serialized: Create a new FILE.new.cdns file by re-serializing the content.
               This is useful to test that round-trip convertion is lossless."#
    );
}
