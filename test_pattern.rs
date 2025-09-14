use regex::Regex;

fn main() {
    let pattern = "Q*::*";
    let name = "QWidget::show";
    
    // Convert glob pattern to regex
    let regex_pattern = pattern
        .replace(".", r"\.")
        .replace("*", ".*")
        .replace("?", ".");
    
    println!("Pattern: {}", pattern);
    println!("Regex: ^{}$", regex_pattern);
    
    let re = Regex::new(&format!("^{}$", regex_pattern)).unwrap();
    let result = re.is_match(name);
    
    println!("Does '{}' match pattern '{}': {}", name, pattern, result);
}