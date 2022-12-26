use crate::languages_cfg::read_cfg;

use fxhash::FxHashMap;
use itertools::Itertools;
use serde::Deserialize;
use arrayvec::ArrayVec;
use nanorand::{Rng, tls_rng};

#[inline]
pub fn shuffle_pins<const N: usize, T>(slice: &mut [T], pins: &[usize]) {
    let mapping: ArrayVec<_, N> = (0..slice.len()).filter(|x| !pins.contains(x)).collect();
	let mut rng = tls_rng();

	for (m, &swap1) in mapping.iter().enumerate() {
        let swap2 = rng.generate_range(m..mapping.len());
        slice.swap(swap1, mapping[swap2]);
    }
}

pub static I_TO_COL: [usize; 30] = [
	0, 1, 2, 3, 3,  4, 4, 5, 6, 7,
	0, 1, 2, 3, 3,  4, 4, 5, 6, 7,
	0, 1, 2, 3, 3,  4, 4, 5, 6, 7
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PosPair(pub usize, pub usize);

const AFFECTS_SCISSOR: [bool; 30] = [
	true,  true,  true,  true,  true,   true,  true,  true,  true,  true,
	true,  true,  false, false, false,  false, false, false, true,  true,
	true,  true,  true,  false, true,   false, false, true,  true,  true
];

impl PosPair {
	pub const fn default() -> Self {
		Self(0, 0)
	}

	pub const fn new(x1: usize, x2: usize) -> Self {
		Self(x1, x2)
	}

	#[inline]
	pub fn affects_scissor(&self) -> bool {
		unsafe {
			*AFFECTS_SCISSOR.get_unchecked(self.0) || *AFFECTS_SCISSOR.get_unchecked(self.1)
		}
	}
}

impl std::fmt::Display for PosPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub const POSSIBLE_SWAPS: [PosPair; 435] = get_possible_swaps();

const fn get_possible_swaps() -> [PosPair; 435] {
	let mut res = [PosPair::default(); 435];
	let mut i = 0;
	let mut pos1 = 0;
	
	while pos1 < 30 {
		let mut pos2 = pos1 + 1;
		while pos2 < 30 {
			res[i] = PosPair(pos1, pos2);
			i += 1;
			pos2 += 1;
		}
		pos1 += 1;
	}
	res
}

#[derive(Clone, Default)]
pub struct ConvertU8 {
	from: Vec<char>,
	to: FxHashMap<char, u8>,
}

impl ConvertU8 {
	pub fn from_single(&self, c: u8) -> char {
		*self.from.get(c as usize).unwrap_or(&' ')
	}

	pub fn from<T>(&self, input: T) -> Vec<char>
	where T: IntoIterator<Item = u8> {
		input.into_iter()
			.map(|c| self.from_single(c))
			.collect()
	}

	pub fn to_single(&mut self, c: char) -> u8 {
		if let Some(u) = self.to.get(&c) {
			*u
		} else {
			let new = self.len();
			self.from.push(c);
			self.to.insert(c, new);
			new
		}
	}

	pub fn to_bigram(&mut self, from: [char; 2]) -> [u8; 2] {
		[self.to_single(from[0]), self.to_single(from[1])]
	}

	pub fn to_trigram(&mut self, from: [char; 3]) -> [u8; 3] {
		[self.to_single(from[0]), self.to_single(from[1]), self.to_single(from[2])]
	}

	pub fn to<T>(&mut self, input: T) -> Vec<u8>
	where T: IntoIterator<Item = char> {
		input.into_iter()
			.map(|c| self.to_single(c))
			.collect()
	}

	pub fn to_single_lossy(&self, c: char) -> u8 {
		if let Some(u) = self.to.get(&c) {
			*u
		} else {
			self.len()
		}
	}

	pub fn to_bigram_lossy(&self, from: [char; 2]) -> [u8; 2] {
		[self.to_single_lossy(from[0]), self.to_single_lossy(from[1])]
	}

	pub fn to_trigram_lossy(&self, from: [char; 3]) -> [u8; 3] {
		[self.to_single_lossy(from[0]), self.to_single_lossy(from[1]), self.to_single_lossy(from[2])]
	}

	pub fn to_lossy<T>(&self, input: T) -> Vec<u8>
	where T: IntoIterator<Item = char> {
		input.into_iter()
			.map(|c| self.to_single_lossy(c))
			.collect()
	}

	pub fn insert_single(&mut self, c: char) {
		if self.to.get(&c).is_none() {
			let new = self.len();
			self.from.push(c);
			self.to.insert(c, new);
		}
	}

	pub fn insert<T>(&mut self, input: T)
	where T: IntoIterator<Item = char> {
		input.into_iter().for_each(|c| self.insert_single(c));
	}

	pub fn with_chars(s: &str) -> Self {
		let mut res = Self::default();
		res.insert(s.chars());
		res
	}

	pub fn as_str(&self, input: &[u8]) -> String {
		input.into_iter()
			.map(|&c| self.from.get(c as usize).unwrap_or(&' '))
			.collect()
	}

	pub fn len(&self) -> u8 {
		debug_assert_eq!(self.to.len(), self.from.len());

		self.to.len() as u8
	}
}

#[derive(Deserialize, Debug)]
pub enum KeyboardType {
	AnsiAngle,
	IsoAngle,
	RowstagDefault,
	Ortho,
	Colstag
}

impl TryFrom<String> for KeyboardType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, &'static str> {
		let lower = value.to_lowercase();
		let split = lower.split_whitespace().collect::<Vec<&str>>();

        if split.len() == 1 {
			match split[0] {
				"ortho" => Ok(Self::Ortho),
				"colstag" => Ok(Self::Colstag),
				"rowstag" | "iso" | "ansi" | "jis" => Ok(Self::RowstagDefault),
				_ => Err("Couldn't parse keyboard type!")
			}
		} else if split.len() == 2 {
			match (split[0], split[1]) {
				("ansi", "angle") => Ok(Self::AnsiAngle),
				("iso", "angle") => Ok(Self::IsoAngle),
				_ => Err("Couldn't parse keyboard type!")
			}
		} else {
			Err("Couldn't parse keyboard type!")
		}
    }
}

pub fn get_effort_map(heatmap_weight: f64, ktype: KeyboardType) -> [f64; 30] {
	use KeyboardType::*;
	
	let mut res = match ktype {
		IsoAngle => [
			3.1, 2.6, 2.0, 2.2, 2.4,  3.3, 2.3, 2.0, 2.6, 3.1,
			1.8, 1.3, 1.1, 1.0, 2.6,  2.6, 1.0, 1.1, 1.3, 1.8,
			3.3, 2.8, 2.4, 1.8, 2.2,  2.2, 1.8, 2.4, 2.8, 3.3
		],
		AnsiAngle => [
			3.0, 2.6, 2.0, 2.2, 2.4,  3.3, 2.3, 2.0, 2.6, 3.0,
			1.8, 1.3, 1.1, 1.0, 2.6,  2.6, 1.0, 1.1, 1.3, 1.8,
			3.7, 2.8, 2.4, 1.8, 2.2,  2.2, 1.8, 2.4, 2.8, 3.3
		],
		RowstagDefault => [
			3.0, 2.6, 2.0, 2.2, 2.4,  3.3, 2.3, 2.0, 2.6, 3.0,
			1.8, 1.3, 1.1, 1.0, 2.6,  2.6, 1.0, 1.1, 1.3, 1.8,
			3.5, 3.0, 2.7, 2.3, 3.7,  2.2, 1.8, 2.4, 2.8, 3.3
		],
		Ortho => [
			3.0, 2.6, 2.0, 2.2, 3.1,  3.1, 2.2, 2.0, 2.6, 3.0,
			1.7, 1.3, 1.1, 1.0, 2.6,  2.6, 1.0, 1.1, 1.3, 1.7,
			3.2, 2.8, 2.3, 1.6, 3.0,  3.0, 1.6, 2.3, 2.8, 3.2
		],
		Colstag => [
			3.0, 2.6, 2.0, 2.2, 3.1,  3.1, 2.2, 2.0, 2.6, 3.0,
			1.7, 1.3, 1.1, 1.0, 2.6,  2.6, 1.0, 1.1, 1.3, 1.7,
			3.4, 2.6, 2.2, 1.8, 3.2,  3.2, 1.8, 2.2, 2.6, 3.4
		],
	};
	
	for i in 0..res.len() {
		res[i] -= 0.2;
		res[i] /= 4.5;
		res[i] *= heatmap_weight;
	}

	res
}

pub fn get_fspeed(lat_multiplier: f64) -> [(PosPair, f64); 48] {
    let mut res = Vec::new();
    for (b, dist) in get_sfb_indices().iter().zip(get_distances(lat_multiplier)) {
        res.push((*b, dist));
    }
    res.try_into().unwrap()
}

fn get_distances(lat_multiplier: f64) -> [f64; 48] {
    let mut res = Vec::new();
    let help = |f: f64, r: f64| f.powi(2).powf(0.65) * r;
    
    for fweight in [1.4, 3.6, 4.8, 4.8, 3.6, 1.4] {
		let ratio = 5.5/fweight;
        res.append(&mut vec![help(1.0, ratio), help(2.0, ratio), help(1.0, ratio)]);
    }

    for _ in 0..2 {
        for c in [
			(0, (0i32, 0)), (1, (0i32, 1)), (2, (0, 2)), (3, (1, 0)), (4, (1, 1)), (5, (1, 2))
		].iter().combinations(2) {
            let (_, xy1) = c[0];
            let (_, xy2) = c[1];

			let x_dist = (xy1.0 - xy2.0) as f64;
			let y_dist = (xy1.1 - xy2.1) as f64;
			let distance = (x_dist.powi(2)*lat_multiplier + y_dist.powi(2)).powf(0.65);
			
			res.push(distance);
        }
    }
    res.try_into().unwrap()
}

pub fn get_sfb_indices() -> [PosPair; 48] {
	let mut res: Vec<PosPair> = Vec::new();
	for i in [0, 1, 2, 7, 8, 9] {
		let chars = [i, i+10, i+20];
		for c in chars.into_iter().combinations(2) {
			res.push(PosPair(c[0], c[1]));
		}
	}
	for i in [0, 2] {
		let chars = [3+i, 13+i, 23+i, 4+i, 14+i, 24+i];
		for c in chars.into_iter().combinations(2) {
			res.push(PosPair(c[0], c[1]));
		}
	}
	res.try_into().unwrap()
}

pub fn get_scissor_indices() -> [PosPair; 17] {
	let mut res: Vec<PosPair> = Vec::new();
	//these two are top pinky to ring homerow
	res.push(PosPair(0, 11));
	res.push(PosPair(9, 18));
	//these two are pinky home to ring bottom
	res.push(PosPair(10, 21));
	res.push(PosPair(19, 28));
	//these four are inner index stretches
	res.push(PosPair(2, 24));
	res.push(PosPair(22, 4));
	res.push(PosPair(5, 27));
	//these add normal stretching between ajacent columns that stretch between 2 rows except for
	//qwerty mi and cr (assuming c is typed with index)
	for i in [0, 1, 2, 6, 7, 8] {
		if i != 2 {
			res.push(PosPair(i, i+21));
		}
		if i != 6 {
			res.push(PosPair(i+1, i+20));
		}
	}
	res.try_into().unwrap()
}

pub fn chars_for_generation(language: &str) -> [char; 30] {
	let languages_cfg_map = read_cfg();

	if let Some(cfg) = languages_cfg_map.get(language) {
		cfg.chars().collect::<Vec<char>>().try_into().unwrap()
	} else {
		let default = languages_cfg_map.get(&String::from("default")).unwrap();
		default.chars().collect::<Vec<char>>().try_into().unwrap()
	}
}

pub trait ApproxEq {
	fn approx_equal(self, other: f64, dec: u8) -> bool;

	fn approx_eq_dbg(self, other: f64, dec: u8) -> bool;
}

impl ApproxEq for f64 {
	fn approx_equal(self, other: f64, dec: u8) -> bool {
		let factor = 10.0f64.powi(dec as i32);
		let a = (self * factor).trunc();
		let b = (other * factor).trunc();
		a == b
	}

	fn approx_eq_dbg(self, other: f64, dec: u8) -> bool {
		let factor = 10.0f64.powi(dec as i32);
		let a = (self * factor).trunc();
		let b = (other * factor).trunc();

		if a != b {
			println!("approx not equal: {self} != {other}");
		}
		a == b
	}
}

pub(crate) fn is_kb_file(entry: &std::fs::DirEntry) -> bool {
	if let Some(ext_os) = entry.path().extension() {
		if let Some(ext) = ext_os.to_str() {
			return ext == "kb"
		}
	}
	false
}

pub(crate) fn layout_name(entry: &std::fs::DirEntry) -> Option<String> {
	if let Some(name_os) = entry.path().file_stem() {
		if let Some(name_str) = name_os.to_str() {
			return Some(name_str.to_string())
		}
	}
	None
}

pub(crate) fn format_layout_str(layout_str: &str) -> String {
	layout_str
		.split("\n")
		.take(3)
		.map(|line| {
			line.split_whitespace()
				.take(10)
				.collect::<String>()
		})
		.collect::<String>()
}

#[cfg(test)]
mod tests {
	use super::*;

	// #[test]
	#[allow(unused)]
	fn shuffle_pinned() {
		let mut rng = tls_rng();
		let mut chars = "abcdefghijklmnopqrstuvwxyz',.;".chars().collect::<Vec<_>>();
		for _ in 0..10000 {
			let pin_count = rng.generate_range(0..30);
			for i in 0..pin_count {
				
			}
		}
	}
}