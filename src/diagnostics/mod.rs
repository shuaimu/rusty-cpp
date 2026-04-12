use colored::*;
use std::fmt;

#[derive(Debug, Clone)]
pub struct BorrowCheckDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub location: Location,
    pub help: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
    #[allow(dead_code)]
    pub span: Option<(usize, usize)>,
}

impl fmt::Display for BorrowCheckDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self.severity {
            Severity::Error => "error".red().bold(),
            Severity::Warning => "warning".yellow().bold(),
            Severity::Note => "note".blue().bold(),
        };

        writeln!(f, "{}: {}", severity_str, self.message.bold())?;

        writeln!(
            f,
            "  {} {}:{}:{}",
            "-->".blue(),
            self.location.file,
            self.location.line,
            self.location.column
        )?;

        if let Some(ref help) = self.help {
            writeln!(f, "{}: {}", "help".green().bold(), help)?;
        }

        for note in &self.notes {
            writeln!(f, "{}: {}", "note".blue(), note)?;
        }

        Ok(())
    }
}

#[allow(dead_code)]
pub fn format_use_after_move(var_name: &str, location: Location) -> BorrowCheckDiagnostic {
    BorrowCheckDiagnostic {
        severity: Severity::Error,
        message: format!("use of moved value: `{}`", var_name),
        location,
        help: Some(format!(
            "consider using `std::move` explicitly or creating a copy"
        )),
        notes: vec![
            format!("value `{}` was moved previously", var_name),
            "once a value is moved, it cannot be used again".to_string(),
        ],
    }
}

#[allow(dead_code)]
pub fn format_double_borrow(var_name: &str, location: Location) -> BorrowCheckDiagnostic {
    BorrowCheckDiagnostic {
        severity: Severity::Error,
        message: format!("cannot borrow `{}` as mutable more than once", var_name),
        location,
        help: Some("consider using a shared reference instead".to_string()),
        notes: vec!["only one mutable borrow is allowed at a time".to_string()],
    }
}

#[allow(dead_code)]
pub fn format_lifetime_error(message: String, location: Location) -> BorrowCheckDiagnostic {
    BorrowCheckDiagnostic {
        severity: Severity::Error,
        message,
        location,
        help: Some("consider adjusting the lifetime annotations".to_string()),
        notes: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = BorrowCheckDiagnostic {
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Location {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
                span: Some((100, 105)),
            },
            help: Some("Try this instead".to_string()),
            notes: vec!["Note 1".to_string()],
        };

        assert!(matches!(diag.severity, Severity::Error));
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.location.line, 10);
        assert_eq!(diag.help, Some("Try this instead".to_string()));
        assert_eq!(diag.notes.len(), 1);
    }

    #[test]
    fn test_format_use_after_move() {
        let location = Location {
            file: "test.cpp".to_string(),
            line: 42,
            column: 8,
            span: None,
        };

        let diag = format_use_after_move("ptr", location);

        assert!(matches!(diag.severity, Severity::Error));
        assert!(diag.message.contains("ptr"));
        assert!(diag.message.contains("moved"));
        assert!(diag.help.is_some());
        assert!(!diag.notes.is_empty());
    }

    #[test]
    fn test_format_double_borrow() {
        let location = Location {
            file: "test.cpp".to_string(),
            line: 15,
            column: 3,
            span: None,
        };

        let diag = format_double_borrow("value", location);

        assert!(matches!(diag.severity, Severity::Error));
        assert!(diag.message.contains("value"));
        assert!(diag.message.contains("mutable"));
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_format_lifetime_error() {
        let location = Location {
            file: "test.cpp".to_string(),
            line: 20,
            column: 10,
            span: Some((200, 210)),
        };

        let diag = format_lifetime_error("Reference outlives its source".to_string(), location);

        assert!(matches!(diag.severity, Severity::Error));
        assert_eq!(diag.message, "Reference outlives its source");
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = BorrowCheckDiagnostic {
            severity: Severity::Error,
            message: "Test error message".to_string(),
            location: Location {
                file: "example.cpp".to_string(),
                line: 5,
                column: 10,
                span: None,
            },
            help: None,
            notes: vec![],
        };

        let output = format!("{}", diag);
        assert!(output.contains("error"));
        assert!(output.contains("Test error message"));
        assert!(output.contains("example.cpp:5:10"));
    }
}
