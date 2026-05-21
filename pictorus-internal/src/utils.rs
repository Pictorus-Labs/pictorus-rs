use alloc::str;
use alloc::string::String;

use core::{convert::Infallible, time::Duration};
use num_traits::{AsPrimitive, Float};

use log::debug;
use serde::{Deserialize, Serialize};

pub struct PictorusVars {
    pub run_path: String,
    pub data_log_rate_hz: f64,
    pub transmit_enabled: bool,
    pub publish_socket: String,
}

// TODO Can we create an error type for these functions? Could we use Option<> instead?
#[allow(clippy::result_unit_err)]
pub fn buffer_to_scalar(buf: &[u8]) -> Result<f64, ()> {
    debug!("Converting buffer to scalar: {buf:?}");
    string_to_scalar(str::from_utf8(buf).or(Err(()))?)
}

#[allow(clippy::result_unit_err)]
pub fn string_to_scalar(val: &str) -> Result<f64, ()> {
    debug!("Converting string to scalar: {val:?}");
    val.trim().parse().or(Err(()))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PictorusError {
    pub err_type: String,
    pub message: String,
}

impl PictorusError {
    pub fn new(err_type: String, message: String) -> Self {
        PictorusError { err_type, message }
    }
}

impl From<Infallible> for PictorusError {
    fn from(_: Infallible) -> Self {
        unreachable!();
    }
}

pub fn positive_duration(f: f64) -> Duration {
    Duration::from_secs_f64(f64::max(0.0, f))
}

pub fn us_to_s<T, U>(time_us: T) -> U
where
    T: AsPrimitive<U> + Copy,
    U: Float + 'static,
{
    let million: U = U::from(1_000_000).unwrap();
    let seconds_part = (time_us.as_() / million).floor();
    let microseconds_remainder = time_us.as_() % million;
    seconds_part + microseconds_remainder / million
}

pub fn s_to_us<T, U>(time_s: T) -> U
where
    T: Float + AsPrimitive<U> + Copy,
    U: AsPrimitive<T> + Copy,
{
    let million: T = T::from(1_000_000).unwrap();
    (time_s * million).as_()
}

pub fn transpose<const N: usize, const M: usize, T: Copy>(input: [[T; N]; M]) -> [[T; M]; N] {
    let mut result = [[input[0][0]; M]; N];

    input.iter().enumerate().for_each(|(i, row)| {
        row.iter().enumerate().for_each(|(j, &val)| {
            result[j][i] = val;
        });
    });
    result
}

// All remaining std-only methods here
#[cfg(feature = "std")]
mod std_utils {
    use super::*;
    use pictorus_traits::Matrix;
    use serde::de::DeserializeOwned;
    use std::collections::HashMap;
    use std::format;
    use std::fs;
    use std::io::Write;
    use std::panic::PanicHookInfo;
    use std::prelude::rust_2021::*;

    use chrono::Local;
    use env_logger::Builder;
    use log::{LevelFilter, info, warn};

    pub type DiagramParams = HashMap<String, HashMap<String, String>>;
    #[derive(serde::Deserialize)]
    struct BigArrayWrap<const N: usize, T: DeserializeOwned>(
        #[serde(with = "serde_big_array::BigArray")] [T; N],
    );

    #[derive(serde::Deserialize)]
    struct BigMatrixWrap<const NROWS: usize, const NCOLS: usize, T: DeserializeOwned>(
        #[serde(with = "serde_big_array::BigArray")] [BigArrayWrap<NCOLS, T>; NROWS],
    );

    // Trait definition for parameters loadable from DiagramParams or env vars
    pub trait LoadableParams: Sized {
        fn parse(source: &str, default: Option<&Self>) -> Option<Self>;
    }

    impl LoadableParams for String {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            Some(source.to_string())
        }
    }

    impl LoadableParams for f64 {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            source.parse().ok()
        }
    }

    impl LoadableParams for Vec<String> {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            let cleaned_str = source.replace(['[', ']', '\"'], "").replace(", ", ",");
            if cleaned_str.is_empty() {
                return None;
            }
            Some(
                cleaned_str
                    .split(',')
                    .filter_map(|s| s.parse::<String>().ok())
                    .collect::<Vec<_>>(),
            )
        }
    }

    impl<const N: usize> LoadableParams for [u8; N]
    where
        BigArrayWrap<N, u8>: serde::de::DeserializeOwned,
    {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            // Try to parse as an array of bytes first
            if let Ok(parsed) = serde_json::from_str::<BigArrayWrap<N, u8>>(source) {
                return Some(parsed.0);
            }
            // handle single element case
            if let Ok(parsed) = serde_json::from_str::<u8>(source)
                .or_else(|_| serde_json::from_str::<[u8; 1]>(source).map(|[val]| val))
            {
                return Some([parsed; N]);
            }
            // Fall back to taking the string as bytes
            // We need to handle `/x**` hex format and turn it into raw bytes
            let escaped_bytes = smashquote::unescape_bytes(source.as_bytes())
                .map_err(|e| warn!("Error unescaping string for bytes: {e}"))
                .ok()?;
            if escaped_bytes.len() != N {
                warn!(
                    "Source string \"{source}\" of length {} is not the expected length of {N}, result will be truncated or padded with zeros",
                    escaped_bytes.len()
                );
            }
            let copy_len = usize::min(escaped_bytes.len(), N);
            let mut array = [0u8; N];
            array[..copy_len].copy_from_slice(&escaped_bytes[..copy_len]);
            Some(array)
        }
    }

    impl<const N: usize> LoadableParams for [f64; N]
    where
        BigArrayWrap<N, f64>: serde::de::DeserializeOwned,
    {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            // Try and parse as a 1d array
            if let Ok(parsed) = serde_json::from_str::<BigArrayWrap<N, f64>>(source) {
                return Some(parsed.0);
            }
            // If that fails check for scalar special case
            // We check for a bare scalar, or a 1d array with 1 element,
            serde_json::from_str::<[f64; 1]>(source)
                .map(|[scalar]| [scalar; N])
                .or_else(|_| serde_json::from_str::<f64>(source).map(|scalar| [scalar; N]))
                .ok()
        }
    }

    impl<const NROWS: usize, const NCOLS: usize> LoadableParams for Matrix<NROWS, NCOLS, f64>
    where
        BigMatrixWrap<NROWS, NCOLS, f64>: serde::de::DeserializeOwned,
    {
        fn parse(source: &str, _default: Option<&Self>) -> Option<Self> {
            // Try and parse as a 2d array
            if let Ok(parsed) = serde_json::from_str::<BigMatrixWrap<NROWS, NCOLS, f64>>(source) {
                let mut matrix = Matrix::zeroed();
                // transpose
                #[allow(clippy::needless_range_loop)]
                for row_i in 0..NROWS {
                    for col_i in 0..NCOLS {
                        matrix.data[col_i][row_i] = parsed.0[row_i].0[col_i];
                    }
                }
                return Some(matrix);
            }
            // If that fails check for scalar special case
            // We check for a bare scalar, a 1x1 matrix, or a 1d array with 1 element,
            // all of which we interpret as scalars that fill the whole matrix
            serde_json::from_str::<[[f64; 1]; 1]>(source)
                .map(|[[scalar]]| scalar)
                .or_else(|_| serde_json::from_str::<[f64; 1]>(source).map(|[scalar]| scalar))
                .or_else(|_| serde_json::from_str::<f64>(source))
                .map(|scalar| Matrix {
                    data: [[scalar; NROWS]; NCOLS],
                })
                .ok()
        }
    }

    // Generic function to load a parameter from env or params
    pub fn load_param<T: LoadableParams + std::fmt::Debug>(
        block_name: &str,
        var_name: &str,
        default: T,
        blocks_map: &DiagramParams,
    ) -> T {
        let env_var_name = format!("{}_{}", block_name.to_uppercase(), var_name.to_uppercase());

        // Try loading from ENV
        if let Ok(env_var) = std::env::var(&env_var_name) {
            if let Some(parsed) = T::parse(&env_var, Some(&default)) {
                info!("Parsing and loading env variable {env_var_name} with value '{parsed:?}'");
                return parsed;
            } else {
                warn!(
                    "Environment variable {env_var_name} was found but could not be parsed, falling back to diagram params or default. Value was: '{env_var}'"
                );
            }
        }

        // Try loading from DiagramParams
        if let Some(params_map) = blocks_map.get(block_name).and_then(|map| map.get(var_name)) {
            if let Some(parsed) = T::parse(params_map, Some(&default)) {
                info!(
                    "Parsing and loading {block_name}_{var_name} from params file with value {parsed:?}"
                );
                return parsed;
            } else {
                warn!(
                    "Found {block_name}_{var_name} in params file but could not parse it, falling back to default. Value was: '{params_map}'"
                );
            }
        }

        // Otherwise return the default
        default
    }

    pub fn get_diagram_params(vars: &PictorusVars) -> DiagramParams {
        // Load diagram variables from diagram_params.json, if present.
        let diagram_params_path =
            std::path::PathBuf::from(&vars.run_path).join("diagram_params.json");
        info!(
            "Looking for diagram params file: {}",
            diagram_params_path.display()
        );
        let params_file = std::fs::read_to_string(diagram_params_path);
        let input_params_json = match params_file {
            Ok(val) => {
                info!("Found params file!");
                val
            }
            Err(_err) => {
                info!("No params file found.");
                String::from("{}")
            }
        };
        serde_json::from_str(input_params_json.as_str()).unwrap_or_else(|_| {
            warn!("Error parsing params file, using empty params map.");
            HashMap::<String, HashMap<String, String>>::new()
        })
    }

    pub fn get_pictorus_vars() -> PictorusVars {
        // Load special environment variables that control app execution, or use safe defaults if not present.
        PictorusVars {
            data_log_rate_hz: std::env::var("APP_DATA_LOG_RATE_HZ")
                .unwrap_or("0".to_string())
                .trim()
                .parse()
                .unwrap(),
            run_path: std::env::var("APP_RUN_PATH").unwrap_or("".to_string()),
            transmit_enabled: std::env::var("APP_TRANSMIT_ENABLED")
                .unwrap_or("true".to_string())
                .parse()
                .unwrap(),
            publish_socket: std::env::var("APP_PUBLISH_SOCKET").unwrap_or("".to_string()),
        }
    }

    pub fn dump_error(err: &PictorusError, run_path: &str) {
        let path = std::path::PathBuf::from(run_path).join("pictorus_errors.json");
        info!("Error log path: {path:?}");
        fs::write(
            path,
            serde_json::to_string(err).expect("Serde JSON Could not parse string"),
        )
        .ok();
    }

    pub fn custom_panic_handler(panic_info: &PanicHookInfo, run_path: &str) {
        warn!("Unhandled panic, dumping stack trace to error log...");
        let err = PictorusError {
            err_type: "unhandled".to_string(),
            message: panic_info.to_string(),
        };
        dump_error(&err, run_path);
    }

    pub fn get_block_type<T>(_: &T) -> String {
        // Pass in a block, get back it's name!
        let name_str = std::any::type_name::<T>().to_string();
        let name_vec: Vec<&str> = name_str.split("::").collect();
        String::from(name_vec[name_vec.len() - 1])
    }

    pub fn initialize_logging() {
        let mut log_level: LevelFilter = LevelFilter::Info;
        if std::env::var("LOG_LEVEL").is_ok() {
            log_level = std::env::var("LOG_LEVEL")
                .unwrap()
                .parse()
                .unwrap_or(LevelFilter::Info);
        }

        Builder::new()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} [{}] - {}",
                    Local::now().format("%+"),
                    record.level(),
                    record.args()
                )
            })
            .filter(None, log_level)
            .init();
        log::info!("Log level: {log_level}");
    }
}
#[cfg(feature = "std")]
pub use std_utils::*;

#[cfg(all(test, feature = "std"))]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;
    use pictorus_traits::Matrix;
    use std::collections::HashMap;
    use temp_env::with_vars;

    #[test]
    fn test_load_param_f64() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "3.14159".to_string());
            params
        });

        let default = 5.0;

        // Test env_var path
        with_vars(vec![("TEST_BLOCK_TEST_VAR", Some("42.0"))], || {
            let result_env = load_param::<f64>("test_block", "test_var", default, &diagram_params);
            assert_eq!(result_env, 42.0);
        });

        // Test diagram_param_path
        let result_param = load_param::<f64>("test_block", "test_var", default, &diagram_params);
        assert_eq!(result_param, 3.14159);

        // Test default path
        let result_default = load_param::<f64>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_string() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "hello".to_string());
            params
        });

        with_vars(vec![("TEST_BLOCK_TEST_VAR", Some("bar"))], || {
            let result_env = load_param::<String>(
                "test_block",
                "test_var",
                "default_string".to_string(),
                &diagram_params,
            );
            assert_eq!(result_env, "bar".to_string());
        });

        let result_param = load_param::<String>(
            "test_block",
            "test_var",
            "default_string".to_string(),
            &diagram_params,
        );

        assert_eq!(result_param, "hello".to_string());

        let result_default = load_param::<String>(
            "test_block",
            "foo",
            "default_string".to_string(),
            &diagram_params,
        );
        assert_eq!(result_default, "default_string".to_string());
    }

    #[test]
    fn test_load_param_vec_string() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert(
                "select_indices".to_string(),
                "[Scalar:2, Scalar:3]".to_string(),
            );
            params
        });

        let default = Vec::from([String::from("Scalar:1"), String::from("BytesArray:3")]);

        with_vars(
            vec![("TEST_BLOCK_TEST_VAR", Some("[Foo, Bar]".to_string()))],
            || {
                let result_env = load_param::<Vec<String>>(
                    "test_block",
                    "test_var",
                    vec!["default_string".to_string()],
                    &diagram_params,
                );
                assert_eq!(result_env, vec!["Foo".to_string(), "Bar".to_string()]);
            },
        );

        let result_param = load_param::<Vec<String>>(
            "test_block",
            "select_indices",
            default.clone(),
            &diagram_params,
        );

        assert_eq!(
            result_param,
            Vec::from([String::from("Scalar:2"), String::from("Scalar:3")])
        );

        let result_default =
            load_param::<Vec<String>>("test_block", "foo", default.clone(), &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_f64() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert(
                "test_var".to_string(),
                "[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]".to_string(),
            );
            params
        });

        let default = [7., 8., 9., 10., 11., 12.];

        with_vars(
            vec![(
                "TEST_BLOCK_TEST_VAR",
                Some("[-1.0, -2.0, -3.0, -4.0, -5.0, -6.0]"),
            )],
            || {
                let result_env =
                    load_param::<[f64; 6]>("test_block", "test_var", default, &diagram_params);
                assert_eq!(result_env, [-1., -2., -3., -4., -5., -6.]);
            },
        );

        let result_param =
            load_param::<[f64; 6]>("test_block", "test_var", default, &diagram_params);

        assert_eq!(result_param, [1., 2., 3., 4., 5., 6.]);

        let result_default = load_param::<[f64; 6]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_f64_scalar_override_vector_default() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "42.0".to_string());
            params
        });

        let default = [1.0, 1.0, 1.0];

        with_vars(vec![("TEST_BLOCK_TEST_VAR", Some("[-13.0]"))], || {
            let result_env =
                load_param::<[f64; 3]>("test_block", "test_var", default, &diagram_params);
            assert_eq!(result_env, [-13., -13., -13.]);
        });

        let result_param =
            load_param::<[f64; 3]>("test_block", "test_var", default, &diagram_params);

        // If override is a scalar of different dimensions from default, use default dimensions
        assert_eq!(result_param, [42.0, 42.0, 42.0]);

        let result_default = load_param::<[f64; 3]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_f64_big_array() {
        let diagram_params = DiagramParams::new();
        let default = [0.0; 37];
        let result_default = load_param::<[f64; 37]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_mat_f64_big_matrix() {
        let diagram_params = DiagramParams::new();
        let default = Matrix {
            data: [[0.0; 37]; 37],
        };
        let result_default =
            load_param::<Matrix<37, 37, f64>>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_u8_big_array() {
        let diagram_params = DiagramParams::new();
        let default = [0u8; 37];
        let result_default = load_param::<[u8; 37]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_u8() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert(
                "test_var".to_string(),
                "[10, 20, 30, 40, 50, 60]".to_string(),
            );
            params
        });

        let default = [0u8, 0, 0, 0, 0, 0];

        with_vars(
            vec![(
                "TEST_BLOCK_TEST_VAR",
                Some("[1, 2, 3, 4, 5, 6]".to_string()),
            )],
            || {
                let result_env =
                    load_param::<[u8; 6]>("test_block", "test_var", default, &diagram_params);
                assert_eq!(result_env, [1, 2, 3, 4, 5, 6]);
            },
        );

        let result_param =
            load_param::<[u8; 6]>("test_block", "test_var", default, &diagram_params);

        assert_eq!(result_param, [10, 20, 30, 40, 50, 60]);

        let result_default = load_param::<[u8; 6]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_u8_scalar_override_vector_default() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "255".to_string());
            params
        });

        let default = [0u8, 0, 0, 0, 0, 0];

        with_vars(
            vec![("TEST_BLOCK_TEST_VAR", Some("[128]".to_string()))],
            || {
                let result_env =
                    load_param::<[u8; 6]>("test_block", "test_var", default, &diagram_params);
                assert_eq!(result_env, [128, 128, 128, 128, 128, 128]);
            },
        );

        let result_param =
            load_param::<[u8; 6]>("test_block", "test_var", default, &diagram_params);

        // If override is a scalar of different dimensions from default, use default dimensions
        assert_eq!(result_param, [255u8; 6]);

        let result_default = load_param::<[u8; 6]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_vec_u8_str_special_case() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "HELLO".to_string());
            params
        });

        let default = *b"hello";

        with_vars(
            vec![("TEST_BLOCK_TEST_VAR", Some("N\\x00ne".to_string()))],
            || {
                let result_env =
                    load_param::<[u8; 5]>("test_block", "test_var", default, &diagram_params);
                assert_eq!(result_env, *b"N\x00ne\x00");
            },
        );

        let result_param =
            load_param::<[u8; 5]>("test_block", "test_var", default, &diagram_params);

        assert_eq!(result_param, *b"HELLO");

        let result_default = load_param::<[u8; 5]>("test_block", "foo", default, &diagram_params);
        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_matrix_f64() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert(
                "test_var".to_string(),
                "[[1.0, 2.0], [3.0, 4.0]]".to_string(),
            );
            params
        });

        let default: Matrix<2, 2, f64> = Matrix {
            data: [[7., 9.], [8., 10.]],
        };

        with_vars(
            vec![("TEST_BLOCK_TEST_VAR", Some("[[3.0, 4.0],[5.0, 6.0]]"))],
            || {
                let result_env = load_param::<Matrix<_, _, _>>(
                    "test_block",
                    "test_var",
                    default,
                    &diagram_params,
                );
                assert_eq!(
                    result_env,
                    Matrix {
                        data: [[3., 5.], [4., 6.]]
                    }
                );
            },
        );

        let result_param =
            load_param::<Matrix<_, _, _>>("test_block", "test_var", default, &diagram_params);

        assert_eq!(
            result_param,
            Matrix {
                data: [[1., 3.], [2., 4.]]
            }
        );

        let result_default =
            load_param::<Matrix<_, _, _>>("test_block", "foo", default, &diagram_params);

        assert_eq!(result_default, default);
    }

    #[test]
    fn test_load_param_matrix_scalar_overide_default() {
        let mut diagram_params = DiagramParams::new();
        diagram_params.insert("test_block".to_string(), {
            let mut params = HashMap::new();
            params.insert("test_var".to_string(), "42.0".to_string());
            params
        });

        let default: Matrix<2, 2, f64> = Matrix {
            data: [[1., 1.], [1., 1.]],
        };

        with_vars(vec![("TEST_BLOCK_TEST_VAR", Some("[[3.0]]"))], || {
            let result_env =
                load_param::<Matrix<_, _, _>>("test_block", "test_var", default, &diagram_params);
            assert_eq!(
                result_env,
                Matrix {
                    data: [[3., 3.], [3., 3.]]
                }
            );
        });

        let result_param =
            load_param::<Matrix<_, _, _>>("test_block", "test_var", default, &diagram_params);

        // If override is a scalar of different dimensions from default, use default dimensions
        assert_eq!(
            result_param,
            Matrix {
                data: [[42., 42.], [42., 42.]]
            }
        );

        let result_default =
            load_param::<Matrix<_, _, _>>("test_block", "foo", default, &diagram_params);

        assert_eq!(result_default, default);
    }

    #[test]
    fn test_positive_duration() {
        assert_eq!(positive_duration(-2.5), Duration::from_secs_f64(0.0));
        assert_eq!(positive_duration(2.5), Duration::from_secs_f64(2.5));
    }

    #[test]
    fn test_us_to_s() {
        assert_eq!(us_to_s::<u64, f64>(1_234_567), 1.234567f64);
    }

    #[test]
    fn test_s_to_us() {
        assert_eq!(s_to_us::<f64, u64>(1.234567f64), 1_234_567);
    }

    #[test]
    fn test_transpose_square_matrix() {
        // Test a square matrix of integers
        let input = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
        let expected = [[1, 4, 7], [2, 5, 8], [3, 6, 9]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_rectangular_matrix() {
        // Test a rectangular matrix of integers (2x3)
        let input = [[1, 2, 3], [4, 5, 6]];
        let expected = [[1, 4], [2, 5], [3, 6]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_rectangular_matrix_reverse() {
        // Test a rectangular matrix of integers (3x2)
        let input = [[1, 2], [3, 4], [5, 6]];
        let expected = [[1, 3, 5], [2, 4, 6]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_single_row() {
        // Test a matrix with a single row
        let input = [[1, 2, 3, 4]];
        let expected = [[1], [2], [3], [4]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_single_column() {
        // Test a matrix with a single column
        let input = [[1], [2], [3], [4]];
        let expected = [[1, 2, 3, 4]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_with_floats() {
        // Test a matrix of floating point numbers
        let input = [[1.1, 2.2], [3.3, 4.4]];
        let expected = [[1.1, 3.3], [2.2, 4.4]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_with_booleans() {
        // Test a matrix of boolean values
        let input = [[true, false], [false, true]];
        let expected = [[true, false], [false, true]];
        assert_eq!(transpose(input), expected);
    }

    #[test]
    fn test_transpose_idempotent() {
        // Test that transposing twice returns the original matrix
        let original = [[1, 2, 3], [4, 5, 6]];
        let transposed = transpose(original);
        let transposed_again = transpose(transposed);
        assert_eq!(transposed_again, original);
    }
}
