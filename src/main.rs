#![allow(clippy::needless_borrow, clippy::wildcard_imports)]

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use json_benchmark::*;

use std::fs::File;
use std::io::{self, Read, Write};

macro_rules! bench {
    {
        name: $name:expr,
        bench: $bench:ident,
        $($args:tt)*
    } => {
        let name = format!(" {} ", $name);
        println!("\n{:=^26} parse|stringify ===== parse|stringify ====", name);

        #[cfg(feature = "file-canada")]
        $bench! {
            path: "data/canada.json",
            structure: canada::Canada,
            $($args)*
        }

        #[cfg(feature = "file-citm-catalog")]
        $bench! {
            path: "data/citm_catalog.json",
            structure: citm_catalog::CitmCatalog,
            $($args)*
        }

        #[cfg(feature = "file-twitter")]
        $bench! {
            path: "data/twitter.json",
            structure: twitter::Twitter,
            $($args)*
        }
    }
}

macro_rules! bench_file {
    {
        path: $path:expr,
        structure: $structure:ty,
        dom: $dom:ty,
        parse_dom: $parse_dom:expr,
        stringify_dom: $stringify_dom:expr,
        $(
            parse_struct: $parse_struct:expr,
            stringify_struct: $stringify_struct:expr,
        )*
    } => {
        let num_trials = num_trials().unwrap_or(256);

        print!("{:22}", $path);
        io::stdout().flush().unwrap();

        let contents = {
            let mut vec = Vec::new();
            File::open($path).unwrap().read_to_end(&mut vec).unwrap();
            vec
        };

        #[cfg(feature = "parse-dom")]
        {
            let dur = timer::bench(num_trials, || {
                let parsed: $dom = $parse_dom(&contents).unwrap();
                parsed
            });
            print!("{:6} MB/s", throughput(dur, contents.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "parse-dom"))]
        print!("          ");

        #[cfg(feature = "stringify-dom")]
        {
            let len = contents.len();
            let dom: $dom = $parse_dom(&contents).unwrap();
            let dur = timer::bench_with_buf(num_trials, len, |out| {
                $stringify_dom(out, &dom).unwrap()
            });
            let mut serialized = Vec::new();
            $stringify_dom(&mut serialized, &dom).unwrap();
            print!("{:6} MB/s", throughput(dur, serialized.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "stringify-dom"))]
        print!("          ");

        $(
            #[cfg(feature = "parse-struct")]
            {
                let dur = timer::bench(num_trials, || {
                    let parsed: $structure = $parse_struct(&contents).unwrap();
                    parsed
                });
                print!("{:6} MB/s", throughput(dur, contents.len()));
                io::stdout().flush().unwrap();
            }
            #[cfg(not(feature = "parse-struct"))]
            print!("          ");

            #[cfg(feature = "stringify-struct")]
            {
                let len = contents.len();
                let parsed: $structure = $parse_struct(&contents).unwrap();
                let dur = timer::bench_with_buf(num_trials, len, |out| {
                    $stringify_struct(out, &parsed).unwrap()
                });
                let mut serialized = Vec::new();
                $stringify_dom(&mut serialized, &parsed).unwrap();
                print!("{:6} MB/s", throughput(dur, serialized.len()));
                io::stdout().flush().unwrap();
            }
        )*

        println!();
    }
}

// This library is handled separately because simd-json needs to mutate its
// input unlike the other libraries. While this makes little difference in a
// real life situation as you're unlikely to deserialize the same data twice,
// it can be a disadvantage in a benchmark.
#[cfg(feature = "lib-simd-json")]
macro_rules! bench_file_simd_json {
    {
        path: $path:expr,
        structure: $structure:ty,
    } => {
        let num_trials = num_trials().unwrap_or(256);

        print!("{:22}", $path);
        io::stdout().flush().unwrap();

        let contents = {
            let mut vec = Vec::new();
            File::open($path).unwrap().read_to_end(&mut vec).unwrap();
            vec
        };

        #[cfg(feature = "parse-dom")]
        {
            use timer::Benchmark;
            let mut benchmark = Benchmark::new();
            let mut data = contents.clone();
            for _ in 0..num_trials {
                data.as_mut_slice().clone_from_slice(contents.as_slice());
                let mut timer = benchmark.start();
                let _parsed = simd_json_parse_dom(&mut data).unwrap();
                timer.stop();
            }
            let dur = benchmark.min_elapsed();
            print!("{:6} MB/s", throughput(dur, contents.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "parse-dom"))]
        print!("          ");

        #[cfg(feature = "stringify-dom")]
        {
            let len = contents.len();
            let mut data = contents.clone();
            let dom = simd_json_parse_dom(&mut data).unwrap();
            let dur = timer::bench_with_buf(num_trials, len, |out| {
                simd_json::Writable::write(&dom, out).unwrap()
            });
            let mut serialized = Vec::new();
            simd_json::Writable::write(&dom, &mut serialized).unwrap();
            print!("{:6} MB/s", throughput(dur, serialized.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "stringify-dom"))]
        print!("          ");

        #[cfg(feature = "parse-struct")]
        {
            use timer::Benchmark;
            let mut benchmark = Benchmark::new();
            let mut data = contents.clone();
            for _ in 0..num_trials {
                data.as_mut_slice().clone_from_slice(contents.as_slice());
                let mut timer = benchmark.start();
                let _parsed: $structure = simd_json_parse_struct(&mut data).unwrap();
                timer.stop();
            }
            let dur = benchmark.min_elapsed();
            print!("{:6} MB/s", throughput(dur, contents.len()));
            io::stdout().flush().unwrap();
        }

        println!();
    }
}

#[cfg(feature = "lib-rmp")]
macro_rules! bench_file_msgpack {
    {
        path: $path:expr,
        structure: $structure:ty,
    } => {
        let num_trials = num_trials().unwrap_or(256);

        print!("{:22}", $path);
        io::stdout().flush().unwrap();

        let contents: Vec<u8> = {
            let structure: $structure = serde_json::from_reader(File::open($path).unwrap()).unwrap();
            rmp_serde::to_vec(&structure).unwrap()
        };

        #[cfg(feature = "parse-dom")]
        {
            use timer::Benchmark;
            let mut benchmark = Benchmark::new();
            for _ in 0..num_trials {
                let mut timer = benchmark.start();
                let _parsed = rmpv::decode::value::read_value(&mut contents.as_slice()).unwrap();
                timer.stop();
            }
            let dur = benchmark.min_elapsed();
            print!("{:6} MB/s", throughput(dur, contents.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "parse-dom"))]
        print!("          ");

        #[cfg(feature = "stringify-dom")]
        {
            let len = contents.len();
            let dom = rmpv::decode::value::read_value(&mut contents.as_slice()).unwrap();
            let dur = timer::bench_with_buf(num_trials, len, |out| {
                rmpv::encode::write_value(out, &dom)
            });
            let mut serialized = Vec::new();
            rmpv::encode::write_value(&mut serialized, &dom).unwrap();
            print!("{:6} MB/s", throughput(dur, serialized.len()));
        }
        #[cfg(not(feature = "stringify-dom"))]
        print!("          ");


        #[cfg(feature = "parse-struct")]
        {
            let dur = timer::bench(num_trials, || {
                let parsed: $structure = rmp_serde::from_slice(&contents).unwrap();
                parsed
            });
            print!("{:6} MB/s", throughput(dur, contents.len()));
            io::stdout().flush().unwrap();
        }
        #[cfg(not(feature = "parse-struct"))]
        print!("          ");

        #[cfg(feature = "stringify-struct")]
        {
            let len = contents.len();
            let parsed: $structure = rmp_serde::from_slice(&contents).unwrap();
            let dur = timer::bench_with_buf(num_trials, len, |out| {
                rmp_serde::encode::write(out, &parsed).unwrap()
            });
            let mut serialized = Vec::new();
            rmp_serde::encode::write(&mut serialized, &parsed).unwrap();
            print!("{:6} MB/s", throughput(dur, serialized.len()));
            io::stdout().flush().unwrap();
        }

        println!();
    };
}

fn main() {
    print!("{:>35}{:>24}", "DOM", "STRUCT");

    #[cfg(feature = "lib-serde")]
    bench! {
        name: "serde_json",
        bench: bench_file,
        dom: serde_json::Value,
        parse_dom: serde_json_parse_dom,
        stringify_dom: serde_json::to_writer,
        parse_struct: serde_json_parse_struct,
        stringify_struct: serde_json::to_writer,
    }

    #[cfg(feature = "lib-rustc-serialize")]
    bench! {
        name: "rustc_serialize",
        bench: bench_file,
        dom: rustc_serialize::json::Json,
        parse_dom: rustc_serialize_parse_dom,
        stringify_dom: rustc_serialize_stringify,
        parse_struct: rustc_serialize_parse_struct,
        stringify_struct: rustc_serialize_stringify,
    }

    #[cfg(feature = "lib-simd-json")]
    bench! {
        name: "simd-json",
        bench: bench_file_simd_json,
    }

    #[cfg(feature = "lib-rmp")]
    bench! {
        name: "rmp",
        bench: bench_file_msgpack,
    }
}

#[cfg(all(
    feature = "lib-serde",
    any(feature = "parse-dom", feature = "stringify-dom")
))]
fn serde_json_parse_dom(bytes: &[u8]) -> serde_json::Result<serde_json::Value> {
    use std::str;
    let s = str::from_utf8(bytes).unwrap();
    serde_json::from_str(s)
}

#[cfg(all(
    feature = "lib-serde",
    any(feature = "parse-struct", feature = "stringify-struct")
))]
fn serde_json_parse_struct<'de, T>(bytes: &'de [u8]) -> serde_json::Result<T>
where
    T: serde::Deserialize<'de>,
{
    use std::str;
    let s = str::from_utf8(bytes).unwrap();
    serde_json::from_str(s)
}

#[cfg(all(
    feature = "lib-rustc-serialize",
    any(feature = "parse-dom", feature = "stringify-dom")
))]
fn rustc_serialize_parse_dom(
    mut bytes: &[u8],
) -> Result<rustc_serialize::json::Json, rustc_serialize::json::BuilderError> {
    rustc_serialize::json::Json::from_reader(&mut bytes)
}

#[cfg(all(
    feature = "lib-rustc-serialize",
    any(feature = "parse-struct", feature = "stringify-struct")
))]
fn rustc_serialize_parse_struct<T>(bytes: &[u8]) -> rustc_serialize::json::DecodeResult<T>
where
    T: rustc_serialize::Decodable,
{
    use std::str;
    let s = str::from_utf8(bytes).unwrap();
    rustc_serialize::json::decode(s)
}

#[cfg(all(
    feature = "lib-rustc-serialize",
    any(feature = "stringify-dom", feature = "stringify-struct")
))]
fn rustc_serialize_stringify<W, T>(writer: W, value: &T) -> rustc_serialize::json::EncodeResult<()>
where
    W: Write,
    T: ?Sized + rustc_serialize::Encodable,
{
    let mut writer = adapter::IoWriteAsFmtWrite::new(writer);
    let mut encoder = rustc_serialize::json::Encoder::new(&mut writer);
    value.encode(&mut encoder)
}

#[cfg(all(
    feature = "lib-simd-json",
    any(feature = "parse-dom", feature = "stringify-dom")
))]
fn simd_json_parse_dom(bytes: &mut [u8]) -> simd_json::Result<simd_json::BorrowedValue> {
    simd_json::to_borrowed_value(bytes)
}

#[cfg(all(
    feature = "lib-simd-json",
    any(feature = "parse-struct", feature = "stringify-struct")
))]
fn simd_json_parse_struct<'de, T>(bytes: &'de mut [u8]) -> simd_json::Result<T>
where
    T: serde::Deserialize<'de>,
{
    simd_json::serde::from_slice(bytes)
}
