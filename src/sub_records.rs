use std;
use std::any::Any;
use std::fs::File;
use std::mem::transmute;

use byteorder::ReadBytesExt;

pub use util::*;

// See http://stackoverflow.com/questions/33687447/how-to-get-struct-reference-from-boxed-trait
// I'd like the default Debug so that AdditionalInfo trait is necessary
// And as_any cannot be derived yet...
pub trait AdditionalInfo: std::fmt::Debug + AdditionalInfoClone { fn as_any(&self) -> &Any; }
impl AdditionalInfo for String { fn as_any(&self) -> &Any { self } }
impl AdditionalInfo for ClassTypeInfoRecord { fn as_any(&self) -> &Any { self } }
impl AdditionalInfo for PrimitiveTypeEnumeration { fn as_any(&self) -> &Any { self } }

// See http://stackoverflow.com/questions/30353462/how-to-clone-a-struct-storing-a-trait-object
pub trait AdditionalInfoClone {
	fn clone_box(&self) -> Box<AdditionalInfo>;
}

impl<T> AdditionalInfoClone for T where T: 'static + AdditionalInfo + Clone {
    fn clone_box(&self) -> Box<AdditionalInfo> {
        Box::new(self.clone())
    }
}

impl Clone for Box<AdditionalInfo> {
    fn clone(&self) -> Box<AdditionalInfo> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfoRecord {
	pub ObjectId: i32,
	pub Name: String,
	pub MemberCount: i32,
	pub MemberNames: Vec<String>,
}

impl ClassInfoRecord {
	pub fn new(file: &mut File) -> Self {
		let oi = read_l_i32(file);
		let name = read_LengthPrefixedString(file);
		let mc = read_l_i32(file);
		let mut v = Vec::with_capacity(mc as usize);
		for _ in 0..mc {
			v.push(read_LengthPrefixedString(file));
		}
		ClassInfoRecord {
			ObjectId: oi,
			Name: name,
			MemberCount: mc,
			MemberNames: v,
		}
	}
}

#[derive(Debug, Clone)]
pub enum PrimitiveTypeEnumeration {
	Boolean=1,
	Byte,
	Char,
	Decimal=5,
	Double,
	Int16,
	Int32,
	Int64,
	SByte,
	Single,
	TimeSpan,
	DateTime,
	UInt16,
	UInt32,
	UInt64,
	Null,
	String,

	Unknown,
}

impl From<u8> for PrimitiveTypeEnumeration {
	fn from(x: u8) -> Self {
		if x >= PrimitiveTypeEnumeration::Unknown as u8 || x == 0 || x == 4 {
			panic!("Invalid PrimitiveTypeEnumeration {:?}", x);
		} else {
			unsafe { transmute(x) }
		}
	}
}

#[derive(Debug, Clone)]
pub enum BinaryTypeEnumeration {
	Primitive,
	String,
	Object,
	SystemClass,
	Class,
	ObjectArray,
	StringArray,
	PrimitiveArray,

	Unknown,
}

impl From<u8> for BinaryTypeEnumeration {
	fn from(x: u8) -> Self {
		if x >= BinaryTypeEnumeration::Unknown as u8 {
			panic!("Invalid BinaryTypeEnumeration {:?}", x);
		} else {
			unsafe { transmute(x) }
		}
	}
}

#[derive(Debug, Clone)]
pub struct ClassTypeInfoRecord {
	TypeName: String,
	LibraryId: i32,
}

impl ClassTypeInfoRecord {
	pub fn new(file: &mut File) -> Self {
		ClassTypeInfoRecord {
			TypeName: read_LengthPrefixedString(file),
			LibraryId: read_l_i32(file),
		}
	}
}

#[derive(Debug, Clone)]
pub struct MemberTypeInfoRecord {
	pub BinaryTypeEnums: Vec<BinaryTypeEnumeration>,
	pub AdditionalInfos: Vec<Option<Box<AdditionalInfo>>>,
}

impl MemberTypeInfoRecord {
	pub fn new(file: &mut File, member_count: usize) -> Self {
		let mut bte_v = Vec::with_capacity(member_count);
		let mut ai_v: Vec<Option<Box<AdditionalInfo>>> = Vec::with_capacity(member_count);

		for _ in 0..member_count {
			bte_v.push(BinaryTypeEnumeration::from(file.read_u8().unwrap()));
		}
		for bte in bte_v.iter() {
			ai_v.push(
				match bte {
					&BinaryTypeEnumeration::Primitive => {
						Some(box(PrimitiveTypeEnumeration::from(file.read_u8().unwrap())))
					}
					&BinaryTypeEnumeration::String => {
						None
					}
					&BinaryTypeEnumeration::SystemClass => {
						Some(box(read_LengthPrefixedString(file)))
					}
					&BinaryTypeEnumeration::Class => {
						Some(box(ClassTypeInfoRecord::new(file)))
					}
					&BinaryTypeEnumeration::StringArray => {
						None
					}
					&BinaryTypeEnumeration::PrimitiveArray => {
						Some(box(PrimitiveTypeEnumeration::from(file.read_u8().unwrap())))
					}
					s @ _ => panic!("Unprocessed AdditionalInfo: {:?}", s),
				}
			);
		}

		MemberTypeInfoRecord {
			BinaryTypeEnums: bte_v,
			AdditionalInfos: ai_v,
		}
	}
}

#[derive(Debug)]
pub enum BinaryArrayTypeEnumeration {
	Single, 
	Jagged, 
	Rectangular, 
	SingleOffset, 
	JaggedOffset, 
	RectangularOffset, 

	Unknown,
}

impl From<u8> for BinaryArrayTypeEnumeration {
	fn from(x: u8) -> Self {
		if x >= BinaryArrayTypeEnumeration::Unknown as u8 {
			panic!("Invalid BinaryArrayTypeEnumeration {:?}", x);
		} else {
			unsafe { transmute(x) }
		}
	}
}

#[derive(Debug, Clone)]
pub struct ArrayInfoRecord {
	ObjectId: i32,
	pub Length: i32,
}

impl ArrayInfoRecord {
	pub fn new(file: &mut File) -> Self {
		ArrayInfoRecord {
			ObjectId: read_l_i32(file),
			Length: read_l_i32(file),
		}
	}
}