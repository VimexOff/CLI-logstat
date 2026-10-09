use std::collections::{BTreeMap, HashMap, HashSet};

use crate::parser::LogEntry;

#[derive(Debug, Default, PartialEq)]
pub struct StatusClasses {
    pub ok: usize,
    pub redirect: usize,
    pub client_err: usize,
    pub server_err: usize,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub total: usize,
    pub invalid: usize,
    pub total_bytes: u64,
    pub classes: StatusClasses,
    // BTreeMap keeps hours sorted, so they print in order.
    pub errors_by_hour: BTreeMap<u8, usize>,
    paths: HashMap<String, usize>,
    methods: HashMap<String, usize>,
    ips: HashSet<String>,
}

impl Stats {
    // Takes the entry by value: after counting it is not needed anymore,
    // so its strings are moved into the maps instead of being cloned.
    pub fn add(&mut self, entry: LogEntry) {
        match entry.status / 100 {
            2 => self.classes.ok += 1,
            3 => self.classes.redirect += 1,
            4 => self.classes.client_err += 1,
            5 => {
                self.classes.server_err += 1;
                if let Some(hour) = entry.hour() {
                    *self.errors_by_hour.entry(hour).or_insert(0) += 1;
                }
            }
            // 1xx and non-standard codes don't belong to any class we report.
            _ => {}
        }

        self.total += 1;
        self.total_bytes += entry.size;
        *self.paths.entry(entry.path).or_insert(0) += 1;
        *self.methods.entry(entry.method).or_insert(0) += 1;
        self.ips.insert(entry.ip);
    }

    pub fn add_invalid(&mut self) {
        self.invalid += 1;
    }

    pub fn unique_ips(&self) -> usize {
        self.ips.len()
    }

    pub fn top_paths(&self, n: usize) -> Vec<(&str, usize)> {
        let mut paths = sorted_counts(&self.paths);
        paths.truncate(n);
        paths
    }

    pub fn methods(&self) -> Vec<(&str, usize)> {
        sorted_counts(&self.methods)
    }
}

/// Most frequent first; ties are sorted by name so the output is the same on every run
/// (HashMap iteration order is random).
fn sorted_counts(map: &HashMap<String, usize>) -> Vec<(&str, usize)> {
    let mut items: Vec<(&str, usize)> = map.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LogParser;

    fn entry(method: &str, path: &str, status: u16, hour: u8) -> LogEntry {
        LogEntry {
            ip: "127.0.0.1".to_string(),
            time: format!("10/Oct/2023:{hour:02}:00:00 +0000"),
            method: method.to_string(),
            path: path.to_string(),
            status,
            size: 100,
        }
    }

    #[test]
    fn counts_status_classes() {
        let mut stats = Stats::default();
        for status in [200, 201, 301, 404, 500, 503, 101] {
            stats.add(entry("GET", "/", status, 10));
        }
        assert_eq!(stats.total, 7);
        assert_eq!(
            stats.classes,
            StatusClasses {
                ok: 2,
                redirect: 1,
                client_err: 1,
                server_err: 2,
            }
        );
    }

    #[test]
    fn top_paths_sorted_and_limited() {
        let mut stats = Stats::default();
        for path in ["/a", "/b", "/b", "/c", "/c", "/c"] {
            stats.add(entry("GET", path, 200, 10));
        }
        assert_eq!(stats.top_paths(2), vec![("/c", 3), ("/b", 2)]);
        assert_eq!(stats.top_paths(10).len(), 3);
    }

    #[test]
    fn server_errors_grouped_by_hour() {
        let mut stats = Stats::default();
        stats.add(entry("GET", "/", 500, 9));
        stats.add(entry("GET", "/", 502, 9));
        stats.add(entry("GET", "/", 503, 23));
        stats.add(entry("GET", "/", 200, 9));
        assert_eq!(stats.errors_by_hour, BTreeMap::from([(9, 2), (23, 1)]));
    }

    #[test]
    fn sample_log_totals() {
        let parser = LogParser::new().expect("regex should compile");
        let mut stats = Stats::default();
        for line in include_str!("../examples/sample.log").lines() {
            match parser.parse(line) {
                Ok(entry) => stats.add(entry),
                Err(_) => stats.add_invalid(),
            }
        }

        assert_eq!(stats.total, 17);
        assert_eq!(stats.invalid, 3);
        assert_eq!(stats.total_bytes, 12559);
        assert_eq!(stats.unique_ips(), 6);
        assert_eq!(
            stats.classes,
            StatusClasses {
                ok: 9,
                redirect: 2,
                client_err: 2,
                server_err: 4,
            }
        );
        assert_eq!(
            stats.top_paths(5),
            vec![
                ("/api/orders", 4),
                ("/api/users", 4),
                ("/", 3),
                ("/index.html", 2),
                ("/admin", 1),
            ]
        );
        assert_eq!(
            stats.methods(),
            vec![("GET", 13), ("POST", 3), ("DELETE", 1)]
        );
        assert_eq!(
            stats.errors_by_hour,
            BTreeMap::from([(13, 1), (14, 2), (15, 1)])
        );
    }
}
