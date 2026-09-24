//! Synthetic price history for the demo portfolio.
//!
//! Prices are drawn as a random walk around a shared market path, so a drawdown shows in every
//! instrument at once and a benchmark comparison is not a straight line. The walk itself runs in
//! `f64` — it is a statistic being generated, not money — and only the daily close it produces
//! crosses into `Decimal`.

use chrono::{Datelike, NaiveDate, Weekday};
use rust_decimal::Decimal;

/// A deterministic generator: the same install always gets the same demo, so a screenshot or a
/// bug report about it describes numbers anybody else can reproduce.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*: three shifts and a multiply, enough shape for a price walk.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Roughly normal with a standard deviation of one: two uniforms summed have a deviation of
    /// 0.408, so the sum is scaled back up — otherwise every `vol` below would be 40% of what it
    /// claims and the demo would have no drawdowns worth drawing.
    fn noise(&mut self) -> f64 {
        (self.unit() + self.unit() - 1.0) * 2.449
    }
}

/// Trading days, oldest first. Quotes only exist on weekdays, which is what the risk metrics
/// assume when they annualise by √252.
pub fn business_days(from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let mut days = Vec::new();
    let mut date = from;
    while date <= to {
        if !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            days.push(date);
        }
        date = date.succ_opt().expect("a date inside chrono's range");
    }
    days
}

/// Daily returns of the market every instrument is correlated with: a steady drift, noise, and
/// one scripted bear market so the drawdown chart and the risk figures have something to report.
pub fn market_path(days: usize) -> Vec<f64> {
    let mut rng = Rng::new(0x5109_0F17);
    let bear_from = days * 9 / 20;
    let bear_to = days * 11 / 20;
    (0..days)
        .map(|i| {
            let shock = if (bear_from..bear_to).contains(&i) {
                -0.0028
            } else {
                0.0
            };
            0.00035 + 0.0075 * rng.noise() + shock
        })
        .collect()
}

/// How one instrument moves: the market's day scaled by `beta`, plus a drift and a noise of its
/// own. Annual figures, divided the way the risk module multiplies them back.
pub struct Shape {
    pub start: f64,
    pub drift: f64,
    pub vol: f64,
    pub beta: f64,
    pub seed: u64,
}

/// One close per trading day, rounded to cents.
pub fn walk(shape: &Shape, market: &[f64]) -> Vec<Decimal> {
    let mut rng = Rng::new(shape.seed);
    let drift = shape.drift / 252.0;
    let vol = shape.vol / 252_f64.sqrt();
    let mut price = shape.start;
    let mut out = Vec::with_capacity(market.len());
    for day in market {
        price *= 1.0 + drift + shape.beta * day + vol * rng.noise();
        price = price.max(shape.start * 0.05);
        out.push(cents(price));
    }
    out
}

/// USD priced in EUR: a slow walk, never far from where it started.
pub fn fx_path(days: usize) -> Vec<Decimal> {
    let mut rng = Rng::new(0xFEED_4EC0);
    let mut rate = 0.93_f64;
    (0..days)
        .map(|_| {
            rate += 0.0012 * rng.noise() + 0.00002 * (0.93 - rate) * 100.0;
            rate = rate.clamp(0.82, 1.02);
            Decimal::from_f64_retain(rate).unwrap_or_default().round_dp(4)
        })
        .collect()
}

fn cents(value: f64) -> Decimal {
    Decimal::from_f64_retain(value).unwrap_or_default().round_dp(2)
}
