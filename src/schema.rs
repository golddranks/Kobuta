use super::KbtError;
use std::error::Error;
use std::str::FromStr;
use std::mem;
use log::{trace, info};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Float32,
    Int32,
}

impl DataType {
    pub fn size(&self) -> u16 {
        (match self {
            DataType::Int32 => mem::size_of::<i32>(),
            DataType::Float32 => mem::size_of::<f32>(),
        } as u16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Data {
    Float32(f32),
    Int32(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub dtype: DataType,
    pub nullable: bool,
}

pub mod literals {
    pub const FLOAT32: &str = "Float32";
    pub const INT32: &str = "Int32";

    pub const NULLABLE: &str = "Nullable";
    pub const SEPARATOR: u8 = b',';
}

const LIT_TYPES: [(&str, DataType); 2] = [
    (literals::FLOAT32, DataType::Float32),
    (literals::INT32, DataType::Int32),
];

pub trait Parse {
    // TODO change the error type AND str parameter
    fn parse(bytes: &str) -> Result<Self, KbtError> where Self: Sized;
    fn write<'o>(&self, output: &'o mut [u8]) -> Result<&'o mut [u8], KbtError>;
}

impl Parse for i32 {
    fn parse(bytes: &str) -> Result<i32, KbtError> {
        i32::from_str_radix(bytes, 10).map_err(|_| KbtError)
    }

    fn write<'o>(&self, output: &'o mut [u8]) -> Result<&'o mut [u8], KbtError> {
        // TODO change the error type
        let bytes =
            itoa::write(&mut *output, *self).map_err(|_| KbtError)?;
        let remainder = &mut output[bytes..];
        Ok(remainder)
    }
}

impl Parse for f32 {
    fn parse(bytes: &str) -> Result<f32, KbtError> {
        f32::from_str(bytes).map_err(|_| KbtError)
    }

    fn write<'o>(&self, output: &'o mut [u8]) -> Result<&'o mut [u8], KbtError> {
        // TODO change the error type
        let bytes = dtoa::write(&mut *output, *self).map_err(|_| KbtError)?;

        let remainder = &mut output[bytes..];
        Ok(remainder)
    }
}


impl Column {

    pub fn parse_data(&self, field: &str) -> Result<Data, Box<Error>> {
        Ok(match self.dtype {
            DataType::Float32 => Data::Float32(f32::parse(field)?),
            DataType::Int32 => Data::Int32(i32::parse(field)?),
        })
    }

    fn parse_single_datatype<'a, 'b>(string: &'a str, literal: &str, datatype: DataType) -> Option<(DataType, &'a str)> {
        if string.starts_with(literal) {
            Some((datatype, string[literal.len()..].trim_left()))
        } else {
            None
        }
    }

    fn parse_datatype(string: &str) -> Result<(DataType, &str), KbtError> {
        (&LIT_TYPES[..]).iter().filter_map(|(literal, datatype)|
                Column::parse_single_datatype(string, literal, *datatype)
            )
            .find(|_| true)
            .ok_or(KbtError)
    }

    fn parse_nullable(string: &str) -> (bool, &str) {
        if string.starts_with(literals::NULLABLE) {
            (true, string[literals::NULLABLE.len()..].trim_left())
        } else {
            (false, string.trim_left())
        }
    }

    pub fn parse(string: &str) -> Result<(Column, &str), KbtError> {

        let (dtype, leftover) = Column::parse_datatype(string)?;

        let (nullable, leftover) = Column::parse_nullable(leftover);

        Ok((Column { dtype, nullable }, leftover))
    }
}

fn parse_separator(string: &str) -> Result<&str, KbtError> {
    if string.as_bytes()[0] == literals::SEPARATOR {
        Ok(string[1..].trim_left())
    } else {
        Err(KbtError)
    }
}

pub fn parse(string: &str) -> Result<Vec<Column>, KbtError> {
    let mut leftover = string.trim();
    let mut columns = vec![];

    loop {
        trace!("leftover: {:?}", leftover);
        let result = Column::parse(leftover)?;
        trace!("result: {:?}", result);
        columns.push(result.0);

        if result.1.is_empty() { break };

        leftover = parse_separator(result.1)?;
    }

    Ok(columns)
}


#[cfg(test)]
mod tests {
    extern crate env_logger;
    use super::*;

    const VALID_SCHEMAS: &[(&str, &[Column])] =
        &[
            ("Float32 Nullable, Int32, Float32", &[
                Column{ dtype: DataType::Float32, nullable: true },
                Column{ dtype: DataType::Int32, nullable: false },
                Column{ dtype: DataType::Float32, nullable: false }
            ]),
            ("Float32", &[
                Column{ dtype: DataType::Float32, nullable: false }
            ]),
            (   "Float32   ", &[
                Column{ dtype: DataType::Float32, nullable: false }
            ]),
            (   "  Int32   Nullable", &[
                Column{ dtype: DataType::Int32, nullable: true }
            ]),
        ];

    #[test]
    fn test_parse_happy_path() {
        env_logger::try_init();
        info!("Starting test_parse_happy_path. {} test cases available.", VALID_SCHEMAS.len());

        for (schema_str, expected_schema) in VALID_SCHEMAS {
            info!("Testing {:?}", schema_str);
            let parsed_schema = parse(schema_str).unwrap();
            assert_eq!(*expected_schema, &*parsed_schema);
        }
    }

    const INVALID_SCHEMAS: &[&str] = &[
        "Float32 Nullable, Int32,, Float32",
        "Float33",
        "Float32 nullable",
        "Float32 Int32",
        "Float32,",
    ];

    #[test]
    fn test_parse_error_path() {
        env_logger::try_init();
        info!("Starting test_parse_error_path. {} test cases available.", INVALID_SCHEMAS.len());

        for schema_str in INVALID_SCHEMAS {
            info!("Testing {:?}", schema_str);
            assert_eq!(Err(KbtError), parse(schema_str));
        }
    }

}


