use std::fmt;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct LogEntry {
    pub ip: String,
    pub time: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub size: u64,
}

impl LogEntry {
    /// Hour of day from a timestamp like `10/Oct/2023:13:55:36 +0000`.
    pub fn hour(&self) -> Option<u8> {
        let (_, rest) = self.time.split_once(':')?;
        rest.get(0..2)?.parse().ok()
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Empty,
    BadFormat,
    BadNumber,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            ParseError::Empty => "empty line",
            ParseError::BadFormat => "line does not match the combined log format",
            ParseError::BadNumber => "status or size is not a valid number",
        };
        write!(f, "{msg}")
    }
}

impl std::error::Error for ParseError {}

pub struct LogParser {
    re: Regex,
}

impl LogParser {
    pub fn new() -> Result<Self, regex::Error> {
        // Only the fields we use are captured; referer and user agent are ignored.
        let re = Regex::new(r#"^(\S+) \S+ \S+ \[([^\]]+)\] "(\S+) (\S+)[^"]*" (\d{3}) (\d+|-)"#)?;
        Ok(LogParser { re })
    }

    pub fn parse(&self, line: &str) -> Result<LogEntry, ParseError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(ParseError::Empty);
        }

        let caps = self.re.captures(line).ok_or(ParseError::BadFormat)?;

        // All groups in the regex are mandatory, so `caps[i]` cannot panic after a match.
        let status = caps[5].parse().map_err(|_| ParseError::BadNumber)?;
        // nginx writes "-" instead of 0 when no body was sent.
        let size = match &caps[6] {
            "-" => 0,
            s => s.parse().map_err(|_| ParseError::BadNumber)?,
        };

        Ok(LogEntry {
            ip: caps[1].to_string(),
            time: caps[2].to_string(),
            method: caps[3].to_string(),
            path: caps[4].to_string(),
            status,
            size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"192.168.1.10 - - [10/Oct/2023:13:55:36 +0000] "GET /index.html HTTP/1.1" 200 2326 "-" "curl/8.0""#;

    fn parser() -> LogParser {
        LogParser::new().expect("regex should compile")
    }

    #[test]
    fn parses_normal_line() {
        let entry = parser().parse(GOOD).expect("line should parse");
        assert_eq!(
            entry,
            LogEntry {
                ip: "192.168.1.10".to_string(),
                time: "10/Oct/2023:13:55:36 +0000".to_string(),
                method: "GET".to_string(),
                path: "/index.html".to_string(),
                status: 200,
                size: 2326,
            }
        );
        assert_eq!(entry.hour(), Some(13));
    }

    #[test]
    fn dash_size_is_zero() {
        let line = r#"10.0.0.1 - - [10/Oct/2023:14:00:00 +0000] "HEAD / HTTP/1.1" 304 - "-" "-""#;
        let entry = parser().parse(line).expect("line should parse");
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn broken_line_is_bad_format() {
        let line = "this is not an nginx log line";
        assert_eq!(parser().parse(line), Err(ParseError::BadFormat));
    }

    #[test]
    fn empty_line_is_empty() {
        assert_eq!(parser().parse(""), Err(ParseError::Empty));
        assert_eq!(parser().parse("   "), Err(ParseError::Empty));
    }
}
