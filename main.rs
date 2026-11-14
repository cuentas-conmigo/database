mod cli;
mod db;

fn main() {
    let mut conn = db::open("cuentas.db").expect("no se pudo abrir cuentas.db");
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::ejecutar(&mut conn, &args) {
        Ok(salida) => println!("{salida}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
