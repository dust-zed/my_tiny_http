use std::str::FromStr;

use ascii::{AsciiString, FromAsciiError};
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn default_reason_phrase(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",

            200 => "Ok",
            201 => "Crated",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            209 => "IM used",

            300 => "Multiple Choice",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",

            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",

            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",

            _ => "Unknown",
        }
    }
}

impl From<i8> for StatusCode {
    fn from(value: i8) -> Self {
        StatusCode(value as u16)
    }
}

impl From<u8> for StatusCode {
    fn from(value: u8) -> Self {
        StatusCode(value as u16)
    }
}

impl From<i16> for StatusCode {
    fn from(value: i16) -> Self {
        StatusCode(value as u16)
    }
}

impl From<u16> for StatusCode {
    fn from(value: u16) -> Self {
        StatusCode(value)
    }
}

impl From<i32> for StatusCode {
    fn from(value: i32) -> Self {
        StatusCode(value as u16)
    }
}

impl From<u32> for StatusCode {
    fn from(value: u32) -> Self {
        StatusCode(value as u16)
    }
}

impl AsRef<u16> for StatusCode {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl PartialEq<u16> for StatusCode {
    fn eq(&self, other: &u16) -> bool {
        &self.0 == other
    }
}

impl PartialEq<StatusCode> for u16 {
    fn eq(&self, other: &StatusCode) -> bool {
        self == &other.0
    }
}

impl PartialOrd<u16> for StatusCode {
    fn partial_cmp(&self, other: &u16) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<StatusCode> for u16 {
    fn partial_cmp(&self, other: &StatusCode) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.0)
    }
}

/// Represent a Response Header
#[derive(Debug, Clone)]
pub struct Header {
    pub field: HeaderField,
    pub value: AsciiString,
}

impl Header {
    #[allow(clippy::result_unit_err)]
    pub fn from_bytes<B1, B2>(header: B1, value: B2) -> Result<Header, ()>
    where
        B1: Into<Vec<u8>> + AsRef<[u8]>,
        B2: Into<Vec<u8>> + AsRef<[u8]>,
    {
        let header = HeaderField::from_bytes(header).or(Err(()))?;
        let value = AsciiString::from_ascii(value).or(Err(()))?;

        Ok(Header {
            field: header,
            value,
        })
    }
}

impl FromStr for Header {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut elems = s.splitn(2, ":");

        let field = elems.next().and_then(|f| f.parse().ok()).ok_or(())?;
        let value = elems
            .next()
            .and_then(|v| AsciiString::from_ascii(v.trim()).ok())
            .ok_or(())?;
        Ok(Header { 
            field, 
            value
        })
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.value.as_str())
    }
}

#[derive(Debug, Clone, Eq)]
struct HeaderField(AsciiString);

impl HeaderField {
    pub fn from_bytes<B>(bytes: B) -> Result<HeaderField, FromAsciiError<B>>
    where
        B: Into<Vec<u8>> + AsRef<[u8]>,
    {
        AsciiString::from_ascii(bytes).map(HeaderField)
    }

    pub fn as_str(&self) -> &AsciiString {
        &self.0
    }

    pub fn equiv(&self, other: &'static str) -> bool {
        other.eq_ignore_ascii_case(self.as_str().as_str())
    }
}

impl FromStr for HeaderField {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(char::is_whitespace) {
            Err(())
        } else {
            AsciiString::from_ascii(s).map(HeaderField).map_err(|_| ())
        }
    }
}

impl Display for HeaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

impl PartialEq for HeaderField {
    fn eq(&self, other: &HeaderField) -> bool {
        let self_str = self.0.as_str();
        let other_str = other.0.as_str();
        self_str.eq_ignore_ascii_case(other_str)
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    ///'GET'
    Get,

    ///'Head'
    Head,

    ///'Post'
    Post,

    ///'PUT'
    Put,

    ///'DELETE'
    Delete,

    /// 'CONNECT'
    Connect,

    /// 'OPTIONS'
    Options,

    /// 'TRACE'
    Trace,

    /// 'PATCH'
    Patch,

    /// Request methods  not standardized by the IETF
    NonStandard(AsciiString)
}

impl Method {
    
    pub fn as_str(&self) -> &str {
        match *self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Patch => "PATCH",
            Self::NonStandard(ref s) => s.as_str()
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "GET" => Self::Get,
            "HEAD" => Self::Head,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "DELETE" => Self::Delete,
            "CONNECT" => Self::Connect,
            "OPTIONS" => Self::Options,
            "TRACE" => Self::Trace,
            "PATCH" => Self::Patch,
            s => {
                let ascii_string = AsciiString::from_ascii(s).map_err(|_| ())?;
                Method::NonStandard(ascii_string)
            }
        })
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// HTTP version (usually 1.0 or 1.1).
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HTTPVersion(pub u8, pub u8);

impl Display for HTTPVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl Ord for HTTPVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let HTTPVersion(my_major, my_minor) = *self;
        let HTTPVersion(other_major, other_minor ) = *other;
        if my_major != other_major {
            return my_major.cmp(&other_major);
        }
        return my_minor.cmp(&other_minor);
    }
}

impl PartialOrd for HTTPVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<(u8, u8)> for HTTPVersion {
    fn eq(&self, other: &(u8, u8)) -> bool {
        self.eq(&HTTPVersion(other.0, other.1))
    }
}

impl PartialEq<HTTPVersion> for (u8, u8) {
    fn eq(&self, other: &HTTPVersion) -> bool {
        let &(major, minor) = self;
        HTTPVersion(major, minor).eq(other)
    }
}

impl PartialOrd<(u8, u8)> for HTTPVersion {
    fn partial_cmp(&self, &(major, minor): &(u8, u8)) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&HTTPVersion(major, minor))
    }
}

impl PartialOrd<HTTPVersion> for (u8, u8) {
    fn partial_cmp(&self, other: &HTTPVersion) -> Option<std::cmp::Ordering> {
        let &(major, minor) = self;
        HTTPVersion(major, minor).partial_cmp(other)
    }
}

impl From<(u8, u8)> for HTTPVersion {
    fn from(value: (u8, u8)) -> Self {
        HTTPVersion(value.0, value.1)
    }
}

#[cfg(test)]
mod test {
    use super::Header;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_parse_header() {
        let header: Header = "Content-Type: text/html".parse().unwrap();
        assert!(header.field.equiv("content-type"));
        assert!(header.value.as_str() == "text/html");
        assert!("hello world".parse::<Header>().is_err());
    }
}