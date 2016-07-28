use std;
use std::fs::File;
use std::any::Any;
use std::mem::transmute;
use std::io::prelude::*;
use std::collections::HashMap;

use byteorder::ReadBytesExt;

pub use sub_records::*;


pub trait Value: std::fmt::Debug { fn as_any(&self) -> &Any; }
impl Value for bool { fn as_any(&self) -> &Any { self } }
impl Value for i32 { fn as_any(&self) -> &Any { self } }
impl Value for f32 { fn as_any(&self) -> &Any { self } }
impl Value for u64 { fn as_any(&self) -> &Any { self } }
impl Value for MemberReferenceRecord { fn as_any(&self) -> &Any { self } }
impl Value for ObjectNullRecord { fn as_any(&self) -> &Any { self } }
impl Value for BinaryObjectStringRecord { fn as_any(&self) -> &Any { self } }
impl Value for ArraySinglePrimitiveRecord { fn as_any(&self) -> &Any { self } }

pub trait Record: std::fmt::Debug {
	fn get_record_type_value() -> u8 where Self: Sized;
}

#[derive(Debug)]
pub enum RecordTypeEnumeration {
	SerializedStreamHeader,
	ClassWithId,
	SystemClassWithMembers,
	ClassWithMembers,
	SystemClassWithMembersAndTypes,
	ClassWithMembersAndTypes,
	BinaryObjectString,
	BinaryArray,
	MemberPrimitiveTyped,
	MemberReference,
	ObjectNull,
	MessageEnd,
	BinaryLibrary,
	ObjectNullMultiple256,
	ObjectNullMultiple,
	ArraySinglePrimitive,
	ArraySingleObject,
	ArraySingleString,
	MethodCall = 21,
	MethodReturn,

	Unknown,
}

impl From<u8> for RecordTypeEnumeration {
	fn from(x: u8) -> Self {
		if x >= RecordTypeEnumeration::Unknown as u8 || (x > 17 && x < 21) {
			panic!("Invalid RecordTypeEnumeration {:?}", x);
		} else {
			unsafe { transmute(x) }
		}
	}
}


#[derive(Debug)]
pub struct SerializationHeaderRecord {
	RootId: i32,
	HeaderId: i32,
	MajorVersion: i32,
	MinorVersion: i32,
}

impl SerializationHeaderRecord {
	pub fn new(file: &mut File) -> Self {
		SerializationHeaderRecord {
			RootId: read_l_i32(file),
			HeaderId: read_l_i32(file),
			MajorVersion: read_l_i32(file),
			MinorVersion: read_l_i32(file),
		}
	}
}

impl Record for SerializationHeaderRecord {
	fn get_record_type_value() -> u8 {
		0
	}
}

#[derive(Debug)]
pub struct ClassWithIdRecord {
	ObjectId: i32,
	pub MetadataId: i32,
}

impl ClassWithIdRecord {
	pub fn new(file: &mut File) -> Self {
		ClassWithIdRecord {
			ObjectId: read_l_i32(file),
			MetadataId: read_l_i32(file),
		}
	}
}

impl Record for ClassWithIdRecord {
	fn get_record_type_value() -> u8 {
		1
	}
}

#[derive(Debug)]
pub enum ClassRecordForClassWithId {
	SystemClassWithMembersAndTypesRecord(SystemClassWithMembersAndTypesRecord),
	ClassWithMembersAndTypesRecord(ClassWithMembersAndTypesRecord),
}

#[derive(Debug, Clone)]
pub struct SystemClassWithMembersAndTypesRecord {
	pub ClassInfo: ClassInfoRecord,
	pub MemberTypeInfo: MemberTypeInfoRecord,
}

impl SystemClassWithMembersAndTypesRecord {
	pub fn new(file: &mut File) -> Self {
		let class_info = ClassInfoRecord::new(file);
		let member_count = class_info.MemberCount as usize;
		let member_type_info = MemberTypeInfoRecord::new(file, member_count);
		SystemClassWithMembersAndTypesRecord {
			ClassInfo: class_info,
			MemberTypeInfo: member_type_info,
		}
	}
}

impl Record for SystemClassWithMembersAndTypesRecord {
	fn get_record_type_value() -> u8 {
		4
	}
}

#[derive(Debug, Clone)]
pub struct ClassWithMembersAndTypesRecord {
	pub ClassInfo: ClassInfoRecord,
	pub MemberTypeInfo: MemberTypeInfoRecord,
	LibraryId: i32,
}

impl ClassWithMembersAndTypesRecord {
	pub fn new(file: &mut File) -> Self {
		let class_info = ClassInfoRecord::new(file);
		let member_count = class_info.MemberCount as usize;
		let member_type_info = MemberTypeInfoRecord::new(file, member_count);
		
		ClassWithMembersAndTypesRecord {
			ClassInfo: class_info,
			MemberTypeInfo: member_type_info,
			LibraryId: read_l_i32(file),
		}
	}
}

impl Record for ClassWithMembersAndTypesRecord {
	fn get_record_type_value() -> u8 {
		5
	}
}

#[derive(Debug, Serialize)]
pub struct BinaryObjectStringRecord {
	pub ObjectId: i32,
	pub Value: String,
}

impl BinaryObjectStringRecord {
	pub fn new(file: &mut File) -> Self {
		BinaryObjectStringRecord {
			ObjectId: read_l_i32(file),
			Value: read_LengthPrefixedString(file),
		}
	}
}
impl Record for BinaryObjectStringRecord {
	fn get_record_type_value() -> u8 {
		6
	}
}

#[derive(Debug)]
pub struct BinaryArrayRecord {
	ObjectId: i32,
	BinaryArrayTypeEnum: BinaryArrayTypeEnumeration,
	Rank: i32,
	Lengths: Vec<i32>,
	LowerBounds: Option<Vec<i32>>,
	TypeEnum: BinaryTypeEnumeration,
	AdditionalTypeInfo: Option<Box<AdditionalInfo>>,
}

impl BinaryArrayRecord {
	pub fn new(file: &mut File) -> Self {
		use sub_records::BinaryArrayTypeEnumeration::*;

		let oi = read_l_i32(file);
		let bat = BinaryArrayTypeEnumeration::from(file.read_u8().unwrap());
		let rank = read_l_i32(file);
		let l = rank as usize;
		let mut lengths = Vec::with_capacity(l);
		for _ in 0..l {
			lengths.push(read_l_i32(file));
		}
		let lower_bounds = match bat {
			SingleOffset | JaggedOffset | RectangularOffset => {
				let mut lower_bounds = Vec::with_capacity(l);
				for _ in 0..l {
					lower_bounds.push(read_l_i32(file));
				}
				Some(lower_bounds)
			},
			Single | Jagged | Rectangular => None,
			Unknown => unreachable!(),
		};

		let type_enum = BinaryTypeEnumeration::from(file.read_u8().unwrap());
		let ati: Option<Box<AdditionalInfo>> = match type_enum {
			BinaryTypeEnumeration::Class => {
				Some(box(ClassTypeInfoRecord::new(file)))
			}
			s @ _ => panic!("Unprocessed AdditionalTypeInfo: {:?}", s),
		};

		BinaryArrayRecord {
			ObjectId: oi,
			BinaryArrayTypeEnum: bat,
			Rank: rank,
			Lengths: lengths,
			LowerBounds: lower_bounds,
			TypeEnum: type_enum,
			AdditionalTypeInfo: ati,
		}
	}
}

impl Record for BinaryArrayRecord {
	fn get_record_type_value() -> u8 {
		7
	}
}

#[derive(Debug, Serialize)]
pub struct MemberReferenceRecord {
	pub IdRef: i32,
}

impl MemberReferenceRecord {
	pub fn new(file: &mut File) -> Self {
		MemberReferenceRecord {
			IdRef: read_l_i32(file),
		}
	}
}

impl Record for MemberReferenceRecord {
	fn get_record_type_value() -> u8 {
		9
	}
}

#[derive(Debug)]
pub struct ObjectNullRecord {}

impl ObjectNullRecord {
	pub fn new() -> Self {
		ObjectNullRecord {}
	}
}
impl Record for ObjectNullRecord {
	fn get_record_type_value() -> u8 {
		10
	}
}


#[derive(Debug)]
pub struct MessageEndRecord {}

impl MessageEndRecord {
	pub fn new() -> Self {
		MessageEndRecord {}
	}
}
impl Record for MessageEndRecord {
	fn get_record_type_value() -> u8 {
		11
	}
}

#[derive(Debug)]
pub struct BinaryLibraryRecord {
	LibraryId: i32,
	LibraryName: String,
}

impl BinaryLibraryRecord {
	pub fn new(file: &mut File) -> Self {
		BinaryLibraryRecord {
			LibraryId: read_l_i32(file),
			LibraryName: read_LengthPrefixedString(file),
		}
	}
}

impl Record for BinaryLibraryRecord {
	fn get_record_type_value() -> u8 {
		12
	}
}

#[derive(Debug)]
pub struct ObjectNullMultiple256Record {
	NullCount: u8,
}

impl ObjectNullMultiple256Record {
	pub fn new(file: &mut File) -> Self {
		ObjectNullMultiple256Record {
			NullCount: file.read_u8().unwrap(),
		}
	}
}

impl Record for ObjectNullMultiple256Record {
	fn get_record_type_value() -> u8 {
		13
	}
}

#[derive(Debug)]
pub struct ObjectNullMultipleRecord {
	NullCount: i32,
}

impl ObjectNullMultipleRecord {
	pub fn new(file: &mut File) -> Self {
		ObjectNullMultipleRecord {
			NullCount: read_l_i32(file),
		}
	}
}

impl Record for ObjectNullMultipleRecord {
	fn get_record_type_value() -> u8 {
		14
	}
}

#[derive(Debug)]
pub struct ArraySinglePrimitiveRecord {
	ArrayInfo: ArrayInfoRecord,
	PrimitiveTypeEnum: PrimitiveTypeEnumeration,
	Values: Vec<i32>,
}

impl ArraySinglePrimitiveRecord {
	pub fn new(file: &mut File) -> Self {
		let ai = ArrayInfoRecord::new(file);
		let pte = PrimitiveTypeEnumeration::from(file.read_u8().unwrap());
		let v = match pte {
			PrimitiveTypeEnumeration::Int32 => {
				let mut v = Vec::with_capacity(ai.Length as usize);
				for _ in 0..(ai.Length as usize) {
					v.push(read_l_i32(file));
				}
				v
			},
			s @ _ => panic!("Unprocessed PrimitiveType: {:?}", s),
		};

		ArraySinglePrimitiveRecord {
			ArrayInfo: ai,
			PrimitiveTypeEnum: pte,
			Values: v,
		}
	}
}

impl Record for ArraySinglePrimitiveRecord {
	fn get_record_type_value() -> u8 {
		15
	}
}

#[derive(Debug)]
pub struct ArraySingleStringRecord {
	ArrayInfo: ArrayInfoRecord,
}

impl ArraySingleStringRecord {
	pub fn new(file: &mut File) -> Self {
		ArraySingleStringRecord {
			ArrayInfo: ArrayInfoRecord::new(file),
		}
	}
}

impl Record for ArraySingleStringRecord {
	fn get_record_type_value() -> u8 {
		17
	}
}

pub fn get_value(file: &mut File, member_type_info: &MemberTypeInfoRecord, member_count: usize, string_map: &mut HashMap<i32, String>) -> Vec<Box<Value>> {
	use sub_records::BinaryTypeEnumeration::*;

	let mut values: Vec<Box<Value>> = Vec::with_capacity(member_count);

	for (binary_type, additional_info) in member_type_info.BinaryTypeEnums.iter().zip(member_type_info.AdditionalInfos.iter()) {
		values.push(
			match binary_type {
				&Primitive => {
					match additional_info.as_ref().unwrap().as_any().downcast_ref::<PrimitiveTypeEnumeration>().unwrap() {
						&PrimitiveTypeEnumeration::Boolean => {
							box(file.read_u8().unwrap() == 1)
						}
						&PrimitiveTypeEnumeration::Int32 => {
							box(read_l_i32(file))
						},
						&PrimitiveTypeEnumeration::Single => {
							box(read_l_f32(file))
						},
						&PrimitiveTypeEnumeration::UInt64 => {
							box(read_l_u64(file))
						},
						s @ _ => panic!("Unprocessed PrimitiveType: {:?}", s),
					}
				},
				&String => {
					match RecordTypeEnumeration::from(file.read_u8().unwrap()) {
						RecordTypeEnumeration::BinaryObjectString => {
							let s = BinaryObjectStringRecord::new(file);
							string_map.insert(s.ObjectId, s.Value.clone());
							box(s)
						}
						RecordTypeEnumeration::MemberReference => {
							box(MemberReferenceRecord::new(file))
						}
						RecordTypeEnumeration::ObjectNull => {
							box(ObjectNullRecord::new())
						}
						s @ _ => {
							println!("pos: {:?}", file.seek(std::io::SeekFrom::Current(0)));
							panic!("Unprocessed ValueTypeEnum: {:?}", s);
						}
					}
				},
				t @ &Class | t @ &SystemClass | t @ &PrimitiveArray | t @ &StringArray => {
					match RecordTypeEnumeration::from(file.read_u8().unwrap()) {
						RecordTypeEnumeration::MemberReference => {
							box(MemberReferenceRecord::new(file))
						}
						RecordTypeEnumeration::ObjectNull => {
							box(ObjectNullRecord::new())
						}
						s @ _ => panic!("Unprocessed ValueTypeEnum: {:?} of {:?}", s, t),
					}
				},
				s @ _ => {
					println!("pos: {:?}", file.seek(std::io::SeekFrom::Current(0)));
					panic!("Unprocessed BinaryTypeEnums: {:?}", s); 	
				}
			}
		);
	}
	values
}