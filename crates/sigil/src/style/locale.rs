use super::strings::{StringKey, Strings};

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

    /// The direction named `ltr` or `rtl`; `None` for any other name.
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
    /// The date order named `ymd`, `dmy`, or `mdy`; `None` for any other name.
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
    /// Whether times show a 12-hour clock with AM and PM.
    pub hour12: bool,
}

impl Default for Locale {
    fn default() -> Self {
        Self::new('.', None)
    }
}

impl Locale {
    /// Creates a locale with these decimal and group marks, Y-M-D dates, and a 24-hour clock.
    #[must_use]
    pub const fn new(decimal: char, group: Option<char>) -> Self {
        Self {
            decimal,
            group,
            group_size: 3,
            zero: '0',
            date_order: DateOrder::Ymd,
            date_separator: '-',
            hour12: false,
        }
    }

    /// The preset locale named `en`, `de`, or `ar`; `None` for any other name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&LOCALE_PRESETS, name)
    }

    /// The English preset: `.` decimal, `,` groups, M/D/Y dates, and a 12-hour clock.
    #[must_use]
    pub const fn en() -> Self {
        let mut locale = Self::new('.', Some(',')).date(DateOrder::Mdy, '/');
        locale.hour12 = true;
        locale
    }

    /// The German preset: `,` decimal, `.` groups, D.M.Y dates, and a 24-hour clock.
    #[must_use]
    pub const fn de() -> Self {
        Self::new(',', Some('.')).date(DateOrder::Dmy, '.')
    }

    /// The Arabic preset: Arabic-Indic marks and zero, D/M/Y dates, and a 12-hour clock.
    #[must_use]
    pub const fn ar() -> Self {
        let mut locale = Self::new('\u{066B}', Some('\u{066C}'))
            .date(DateOrder::Dmy, '/')
            .zero('\u{0660}');
        locale.hour12 = true;
        locale
    }

    /// Sets the order of the date parts and the character between them.
    #[must_use]
    pub const fn date(mut self, order: DateOrder, separator: char) -> Self {
        self.date_order = order;
        self.date_separator = separator;
        self
    }

    /// Sets the character for the digit zero.
    #[must_use]
    pub const fn zero(mut self, zero: char) -> Self {
        self.zero = zero;
        self
    }

    /// Sets the number of digits in each group; the default is 3.
    #[must_use]
    pub const fn group_size(mut self, size: u8) -> Self {
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
        let separator = self.date_separator.to_string();
        let out = self
            .date_order
            .parts()
            .map(|part| match part {
                DatePart::Year => format!("{year:04}"),
                DatePart::Month => format!("{month:02}"),
                DatePart::Day => format!("{day:02}"),
            })
            .join(&separator);
        out.chars().map(|c| self.local_digit(c)).collect()
    }

    /// Parses a date in this locale's order.
    #[must_use]
    pub fn parse_date(&self, input: &str) -> Option<(i32, u8, u8)> {
        let ascii = self.ascii_digits(input);
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

    /// Formats a time of day as `hour:minute` or `hour:minute:second`, on a 12-hour clock with the `Am` and `Pm` texts of `strings` when `hour12` is set.
    #[must_use]
    pub fn format_time(
        &self,
        hour: u8,
        minute: u8,
        second: Option<u8>,
        strings: &Strings,
    ) -> String {
        let hour = hour % 24;
        let clock = if self.hour12 {
            format!("{}:{minute:02}", (hour + 11) % 12 + 1)
        } else {
            format!("{hour:02}:{minute:02}")
        };
        let clock = match second {
            Some(second) => format!("{clock}:{second:02}"),
            None => clock,
        };
        let mut out: String = clock.chars().map(|c| self.local_digit(c)).collect();
        if self.hour12 {
            out.push(' ');
            out.push_str(strings.get(if hour < 12 {
                StringKey::Am
            } else {
                StringKey::Pm
            }));
        }
        out
    }

    /// Parses a time of day as 24-hour `hour:minute[:second]`, or 12-hour with the `Am` or `Pm` text of `strings` as a suffix.
    #[must_use]
    pub fn parse_time(&self, input: &str, strings: &Strings) -> Option<(u8, u8, Option<u8>)> {
        let upper = self.ascii_digits(input).to_uppercase();
        let trimmed = upper.trim_end();
        let (clock, pm) = [(StringKey::Pm, true), (StringKey::Am, false)]
            .into_iter()
            .find_map(|(key, pm)| {
                let suffix = strings.get(key).to_uppercase();
                (!suffix.is_empty())
                    .then(|| trimmed.strip_suffix(suffix.as_str()))
                    .flatten()
                    .map(|rest| (rest, Some(pm)))
            })
            .unwrap_or((trimmed, None));
        let mut fields = clock.trim().split(':');
        let hour = fields.next()?.trim().parse::<u8>().ok()?;
        let minute = two_digits(fields.next()?)?;
        let second = match fields.next() {
            Some(field) => Some(two_digits(field)?),
            None => None,
        };
        if fields.next().is_some() {
            return None;
        }
        let hour = match pm {
            Some(pm) if (1..=12).contains(&hour) => hour % 12 + if pm { 12 } else { 0 },
            Some(_) => return None,
            None => hour,
        };
        Some((hour, minute, second))
    }

    fn ascii_digits(&self, input: &str) -> String {
        input
            .trim()
            .chars()
            .map(|c| self.digit_value(c).unwrap_or(c))
            .collect()
    }
}

fn two_digits(field: &str) -> Option<u8> {
    let field = field.trim();
    (field.len() == 2).then(|| field.parse().ok()).flatten()
}
