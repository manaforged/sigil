use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

impl Direction {
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::Rtl)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DateOrder {
    #[default]
    Ymd,
    Dmy,
    Mdy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatePart {
    Year,
    Month,
    Day,
}

impl DateOrder {
    const fn parts(self) -> [DatePart; 3] {
        match self {
            Self::Ymd => [DatePart::Year, DatePart::Month, DatePart::Day],
            Self::Dmy => [DatePart::Day, DatePart::Month, DatePart::Year],
            Self::Mdy => [DatePart::Month, DatePart::Day, DatePart::Year],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Locale {
    pub decimal: char,
    pub group: Option<char>,
    pub group_size: u8,
    pub zero: char,
    pub date_order: DateOrder,
    pub date_separator: char,
}

impl Default for Locale {
    fn default() -> Self {
        Self::new('.', None)
    }
}

impl Locale {
    #[must_use]
    pub const fn new(decimal: char, group: Option<char>) -> Self {
        Self {
            decimal,
            group,
            group_size: 3,
            zero: '0',
            date_order: DateOrder::Ymd,
            date_separator: '-',
        }
    }

    #[must_use]
    pub const fn en() -> Self {
        Self::new('.', Some(',')).with_date(DateOrder::Mdy, '/')
    }

    #[must_use]
    pub const fn de() -> Self {
        Self::new(',', Some('.')).with_date(DateOrder::Dmy, '.')
    }

    #[must_use]
    pub const fn ar() -> Self {
        Self::new('\u{066B}', Some('\u{066C}'))
            .with_date(DateOrder::Dmy, '/')
            .with_zero('\u{0660}')
    }

    #[must_use]
    pub const fn with_date(mut self, order: DateOrder, separator: char) -> Self {
        self.date_order = order;
        self.date_separator = separator;
        self
    }

    #[must_use]
    pub const fn with_zero(mut self, zero: char) -> Self {
        self.zero = zero;
        self
    }

    #[must_use]
    pub const fn with_group_size(mut self, size: u8) -> Self {
        self.group_size = size;
        self
    }

    fn digit_value(self, c: char) -> Option<char> {
        let offset = u32::from(c).checked_sub(u32::from(self.zero))?;
        (offset < 10)
            .then(|| char::from_digit(offset, 10))
            .flatten()
    }

    fn local_digit(self, c: char) -> char {
        c.to_digit(10)
            .and_then(|d| char::from_u32(u32::from(self.zero) + d))
            .unwrap_or(c)
    }

    #[must_use]
    pub fn normalize_number(&self, input: &str) -> Option<String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut out = String::with_capacity(trimmed.len());
        for c in trimmed.chars() {
            if Some(c) == self.group {
                continue;
            }
            if c == self.decimal {
                out.push('.');
            } else if let Some(d) = self.digit_value(c) {
                out.push(d);
            } else if c == '.' && self.decimal != '.' {
                return None;
            } else {
                out.push(c);
            }
        }
        Some(out)
    }

    #[must_use]
    pub fn parse_number(&self, input: &str) -> Option<f64> {
        self.normalize_number(input)?
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    }

    #[must_use]
    pub fn localize_number(&self, plain: &str) -> String {
        plain
            .chars()
            .map(|c| {
                if c == '.' {
                    self.decimal
                } else {
                    self.local_digit(c)
                }
            })
            .collect()
    }

    #[must_use]
    pub fn format_number(&self, value: f64, fraction_digits: usize) -> String {
        let plain = format!("{:.*}", fraction_digits, value.abs());
        let (whole, fraction) = plain.split_once('.').unwrap_or((plain.as_str(), ""));
        let mut out = String::with_capacity(plain.len() + whole.len() / 3 + 1);
        if value.is_sign_negative() && value != 0.0 {
            out.push('-');
        }
        out.push_str(&self.group_digits(whole));
        if !fraction.is_empty() {
            out.push(self.decimal);
            out.extend(fraction.chars().map(|c| self.local_digit(c)));
        }
        out
    }

    fn group_digits(&self, whole: &str) -> String {
        let size = usize::from(self.group_size.max(1));
        let len = whole.chars().count();
        let mut out = String::with_capacity(len + len / size);
        for (i, c) in whole.chars().enumerate() {
            if let Some(group) = self.group
                && i > 0
                && (len - i).is_multiple_of(size)
            {
                out.push(group);
            }
            out.push(self.local_digit(c));
        }
        out
    }

    #[must_use]
    pub fn format_date(&self, year: i32, month: u8, day: u8) -> String {
        let mut out = String::with_capacity(10);
        for (index, part) in self.date_order.parts().into_iter().enumerate() {
            if index > 0 {
                out.push(self.date_separator);
            }
            let _ = match part {
                DatePart::Year => write!(out, "{year:04}"),
                DatePart::Month => write!(out, "{month:02}"),
                DatePart::Day => write!(out, "{day:02}"),
            };
        }
        out.chars().map(|c| self.local_digit(c)).collect()
    }

    #[must_use]
    pub fn parse_date(&self, input: &str) -> Option<(i32, u8, u8)> {
        let ascii: String = input
            .trim()
            .chars()
            .map(|c| self.digit_value(c).unwrap_or(c))
            .collect();
        let mut fields = ascii.splitn(3, self.date_separator);
        let (mut year, mut month, mut day) = (None, None, None);
        for part in self.date_order.parts() {
            let field = fields.next()?.trim();
            match part {
                DatePart::Year => year = field.parse::<i32>().ok(),
                DatePart::Month => month = field.parse::<u8>().ok(),
                DatePart::Day => day = field.parse::<u8>().ok(),
            }
        }
        Some((year?, month?, day?))
    }
}
