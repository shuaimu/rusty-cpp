pub fn parameter(adapted: usize) -> usize {
    adapted
}

pub fn local() -> usize {
    let adapted = 7usize;
    adapted
}

pub fn closure() -> usize {
    let increment = |adapted: usize| adapted + 1;
    increment(7)
}

pub fn matched(value: Option<usize>) -> usize {
    match value {
        Some(adapted) => adapted,
        None => 0,
    }
}

pub fn looped(values: Vec<usize>) -> usize {
    let mut total = 0;
    for adapted in values {
        total += adapted;
    }
    total
}

pub struct Receiver;

impl Receiver {
    pub fn choose(&self) -> usize {
        11
    }
}

pub struct Other;

impl Other {
    pub fn choose() -> usize {
        13
    }
}

pub fn receiver_method_tail_is_not_an_associated_path(receiver: &Receiver) -> usize {
    receiver.choose()
}

pub fn local_associated_path() -> usize {
    Other::choose()
}

pub fn if_let_binding(value: Option<usize>) -> usize {
    if let Some(r#adapted) = value {
        assert!(r#adapted < 1024);
        r#adapted
    } else {
        0
    }
}

pub fn chained_if_let_binding(value: Option<usize>) -> usize {
    if let Some(r#adapted) = value
        && let Some(next) = Some(r#adapted + 1)
        && next > 0
    {
        next
    } else {
        0
    }
}

pub fn chained_while_let_binding(mut values: Vec<Option<usize>>) -> usize {
    let mut total = 0;
    while let Some(Some(r#adapted)) = values.pop()
        && r#adapted > 0
    {
        total += r#adapted;
    }
    total
}
