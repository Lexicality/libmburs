/*
 * Copyright 2023 Lexi Robinson
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use libmbus::parse::Datagram;
use std::error;

fn do_file(fname: &str) -> Result<(), Box<dyn error::Error>> {
    let data = std::fs::read(fname).map_err(Box::new)?;
    let _ = Datagram::parse(data).map_err(Box::new)?;
    Ok(())
}

fn main() {
    for fname in std::env::args().skip(1) {
        println!("Trying to load file {}", fname);
        let res = do_file(&fname);
        match res {
            Ok(_) => println!("Yay"),
            Err(e) => eprintln!("Oh no: {}", e),
        }
    }
    // honk
}
