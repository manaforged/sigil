use std::fmt::Write as _;

/// Layout direction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Direction {
    /// The ltr direction.
    #[default]
    Ltr,
    /// The rtl direction.
    Rtl,
}

impl Direction {
    /// Returns whether this direction is rtl.
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::Rtl)
    }

    /// Creates a direction from name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&DIRECTION_NAMES, name)
    }
}

const DIRECTION_NAMES: [(&str, Direction); 2] = [("ltr", Direction::Ltr), ("rtl", Direction::Rtl)];

const DATE_ORDER_NAMES: [(&str, DateOrder); 3] = [
    ("ymd", DateOrder::Ymd),
    ("dmy", DateOrder::Dmy),
    ("mdy", DateOrder::Mdy),
];

const LOCALE_PRESETS: [(&str, Locale); 3] = [
    ("en", Locale::en()),
    ("de", Locale::de()),
    ("ar", Locale::ar()),
];

fn lookup<T: Copy>(table: &[(&str, T)], name: &str) -> Option<T> {
    table
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, value)| *value)
}

/// The order of day, month, and year in a date.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DateOrder {
    /// The ymd date order.
    #[default]
    Ymd,
    /// The dmy date order.
    Dmy,
    /// The mdy date order.
    Mdy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatePart {
    Year,
    Month,
    Day,
}

impl DateOrder {
    /// Creates a date order from name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&DATE_ORDER_NAMES, name)
    }

    const fn parts(self) -> [DatePart; 3] {
        match self {
            Self::Ymd => [DatePart::Year, DatePart::Month, DatePart::Day],
            Self::Dmy => [DatePart::Day, DatePart::Month, DatePart::Year],
            Self::Mdy => [DatePart::Month, DatePart::Day, DatePart::Year],
        }
    }
}

/// The number and date formats of a language.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Locale {
    /// The decimal.
    pub decimal: char,
    /// The group, if any.
    pub group: Option<char>,
    /// The group size.
    pub group_size: u8,
    /// The zero.
    pub zero: char,
    /// The date order.
    pub date_order: DateOrder,
    /// The date separator.
    pub date_separator: char,
}

impl Default for Locale {
    fn default() -> Self {
        Self::new('.', None)
    }
}

impl Locale {
    /// Creates a locale.
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

    /// Creates a locale from name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&LOCALE_PRESETS, name)
    }

    /// Returns the en locale.
    #[must_use]
    pub const fn en() -> Self {
        Self::new('.', Some(',')).with_date(DateOrder::Mdy, '/')
    }

    /// Returns the de locale.
    #[must_use]
    pub const fn de() -> Self {
        Self::new(',', Some('.')).with_date(DateOrder::Dmy, '.')
    }

    /// Returns the ar locale.
    #[must_use]
    pub const fn ar() -> Self {
        Self::new('\u{066B}', Some('\u{066C}'))
            .with_date(DateOrder::Dmy, '/')
            .with_zero('\u{0660}')
    }

    /// Sets the date order.
    #[must_use]
    pub const fn with_date(mut self, order: DateOrder, separator: char) -> Self {
        self.date_order = order;
        self.date_separator = separator;
        self
    }

    /// Sets the zero digit.
    #[must_use]
    pub const fn with_zero(mut self, zero: char) -> Self {
        self.zero = zero;
        self
    }

    /// Sets the group size.
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

    /// Converts localized number text to plain digits.
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

    /// Parses localized number text.
    #[must_use]
    pub fn parse_number(&self, input: &str) -> Option<f64> {
        self.normalize_number(input)?
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    }

    /// Converts plain number text to this locale's separators.
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

    /// Formats `value` with this many fraction digits.
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

    /// Formats a date in this locale's order.
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

    /// Parses a date in this locale's order.
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
