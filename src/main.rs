//! https://stackoverflow.com/questions/3052202/how-to-analyse-contents-of-binary-serialization-stream

#![feature(box_syntax, custom_derive, plugin)]
#![plugin(serde_macros)]

#![allow(non_snake_case, dead_code)]

extern crate byteorder;
extern crate serde_json;
extern crate itertools;

mod util;
mod sub_records;
mod records;

use std::io::prelude::*;
use std::path::Path;
use std::fs::OpenOptions;
use std::collections::HashMap;

use byteorder::ReadBytesExt;
use serde_json::{Value as JValue, Map, to_value, to_writer_pretty};
use itertools::Zip;

use records::*;


fn main() {
	use records::RecordTypeEnumeration::*;

	let parse_bool = |value: &Box<Value>| {
			JValue::Bool(*value.as_any().downcast_ref::<bool>().unwrap())
		};
		let parse_i32 = |value: &Box<Value>| {
			JValue::I64(*value.as_any().downcast_ref::<i32>().unwrap() as i64)
		};
		let parse_f32 = |value: &Box<Value>| {
			JValue::F64(*value.as_any().downcast_ref::<f32>().unwrap() as f64)
		};
		let parse_u64 = |value: &Box<Value>| {
			JValue::U64(*value.as_any().downcast_ref::<u64>().unwrap())
		};
		
		let parse_MemberReferenceRecord_or_ObjectNullRecord = |value: &Box<Value>| {
			match value.as_any().downcast_ref::<MemberReferenceRecord>() {
				Some(mem_ref) => to_value(mem_ref),
				None => JValue::Null,
			}
		};

	for path_str in std::env::args().skip(1) {
		let path = Path::new(&path_str);
		if path.extension().unwrap() != "bytes" {
			continue;
		}
		let mut file = OpenOptions::new().read(true).open(path).unwrap();
		let mut metadata_vec: Vec<Box<Record>> = vec![];
		let mut id_to_class = HashMap::new();
		let mut id_and_values_vec = vec![];
		let mut string_map = HashMap::<i32, String>::new();
		loop {
			match RecordTypeEnumeration::from(file.read_u8().unwrap()) {
				SerializedStreamHeader => {
					metadata_vec.push(box(SerializationHeaderRecord::new(&mut file)));
				},
				ClassWithId => {
					let boxed_class_with_id = box(ClassWithIdRecord::new(&mut file));
					let class_id = boxed_class_with_id.MetadataId;
					match id_to_class.get(&class_id).unwrap() {
						&ClassRecordForClassWithId::ClassWithMembersAndTypesRecord(ref class) => {
							id_and_values_vec.push((class_id, get_value(&mut file, &class.MemberTypeInfo, class.ClassInfo.MemberCount as usize, &mut string_map)));
						},
						&ClassRecordForClassWithId::SystemClassWithMembersAndTypesRecord(ref class) => {
							id_and_values_vec.push((class_id, get_value(&mut file, &class.MemberTypeInfo, class.ClassInfo.MemberCount as usize, &mut string_map)));
						},
					}

					metadata_vec.push(boxed_class_with_id);
				},
				SystemClassWithMembersAndTypes => {
					let class = SystemClassWithMembersAndTypesRecord::new(&mut file);
					let class_id = class.ClassInfo.ObjectId;
					id_and_values_vec.push((class_id, get_value(&mut file, &class.MemberTypeInfo, class.ClassInfo.MemberCount as usize, &mut string_map)));
					id_to_class.insert(class_id, ClassRecordForClassWithId::SystemClassWithMembersAndTypesRecord(class.clone()));
					metadata_vec.push(box(class)); 
				}
				ClassWithMembersAndTypes => {
					let class = ClassWithMembersAndTypesRecord::new(&mut file);
					let class_id = class.ClassInfo.ObjectId;
					id_and_values_vec.push((class_id, get_value(&mut file, &class.MemberTypeInfo, class.ClassInfo.MemberCount as usize, &mut string_map)));
					id_to_class.insert(class_id, ClassRecordForClassWithId::ClassWithMembersAndTypesRecord(class.clone()));
					metadata_vec.push(box(class)); 
				}
				RecordTypeEnumeration::BinaryObjectString => {
					let s = BinaryObjectStringRecord::new(&mut file);
					string_map.insert(s.ObjectId, s.Value.clone());
					metadata_vec.push(box(s));
				}
				BinaryArray => {
					metadata_vec.push(box(BinaryArrayRecord::new(&mut file)));
				}
				MemberReference => {
					metadata_vec.push(box(MemberReferenceRecord::new(&mut file)));
				}
				ObjectNull => {
					metadata_vec.push(box(ObjectNullRecord::new()));
				}
				MessageEnd => {
					metadata_vec.push(box(MessageEndRecord::new()));
					break;
				}
				BinaryLibrary => {
					metadata_vec.push(box(BinaryLibraryRecord::new(&mut file)));
				}
				ObjectNullMultiple256 => {
					metadata_vec.push(box(ObjectNullMultiple256Record::new(&mut file)));
				}
				ObjectNullMultiple => {
					metadata_vec.push(box(ObjectNullMultipleRecord::new(&mut file)));
				}
				ArraySinglePrimitive => {
					metadata_vec.push(box(ArraySinglePrimitiveRecord::new(&mut file)));
				}
				ArraySingleString => {
					metadata_vec.push(box(ArraySingleStringRecord::new(&mut file)));
				}
				s @ _ => {
					println!("pos: {:?}", file.seek(std::io::SeekFrom::Current(0)));
					println!("Unprocessed RecordTypeEnumeration: {:?}", s);
					break;
				}
			}
		}

		assert_eq!(file.seek(std::io::SeekFrom::Current(0)).unwrap(), file.metadata().unwrap().len());


		let parent_dir = path.parent().unwrap();
		let filename = path.file_name().unwrap();
		// let metadata_path = parent_dir.join(Path::new(filename).with_extension("metadata"));
		// let mut metadata_file = OpenOptions::new().write(true).create(true).truncate(true).open(metadata_path).unwrap();
		// write!(metadata_file, "{:#?}", metadata_vec).unwrap();

		let get_class_info = |class_id| {
			match id_to_class.get(&class_id).unwrap() {
				&ClassRecordForClassWithId::SystemClassWithMembersAndTypesRecord(ref c) => 
					(c.ClassInfo.Name.clone(), &c.ClassInfo.MemberNames, &c.MemberTypeInfo.BinaryTypeEnums, &c.MemberTypeInfo.AdditionalInfos),
				&ClassRecordForClassWithId::ClassWithMembersAndTypesRecord(ref c) => 
					(c.ClassInfo.Name.clone(), &c.ClassInfo.MemberNames, &c.MemberTypeInfo.BinaryTypeEnums, &c.MemberTypeInfo.AdditionalInfos),
			}
		};

		let parse_String = |value: &Box<Value>| {
			// Try BinaryObjectStringRecord first as it seems more
			match value.as_any().downcast_ref::<BinaryObjectStringRecord>() {
				Some(s) => {
					// string_map.insert(s.ObjectId, s.Value.clone());//rustc Error
					JValue::String(s.Value.clone())
				},
				None => {
					match value.as_any().downcast_ref::<MemberReferenceRecord>() {
						Some(mem_ref) => {
							match string_map.get(&mem_ref.IdRef) {
								Some(s) => JValue::String(s.clone()),
								None => to_value(mem_ref),
							}
						}
						None => {
							// let null = value.as_any().downcast_ref::<ObjectNullRecord>().unwrap();
							JValue::Null
						}
					}
				}
			}
		};

		let create_parse_class_vec = |binary_types, additional_infos: &Vec<Option<Box<AdditionalInfo>>>| {
			let mut parse_class_vec: Vec<Box<Fn(&Box<Value>) -> JValue>> = vec![];
			for (binary_type, additional_info) in Zip::new((binary_types, additional_infos)) {
				use records::BinaryTypeEnumeration::*;
				match binary_type {
					&Primitive => {
						match additional_info.as_ref().unwrap().as_any().downcast_ref::<PrimitiveTypeEnumeration>().unwrap() {
							&PrimitiveTypeEnumeration::Boolean => parse_class_vec.push(box(&parse_bool)),
							&PrimitiveTypeEnumeration::Int32 => parse_class_vec.push(box(&parse_i32)),
							&PrimitiveTypeEnumeration::Single => parse_class_vec.push(box(&parse_f32)),
							&PrimitiveTypeEnumeration::UInt64 => parse_class_vec.push(box(&parse_u64)),
							s @ _ => panic!("{:?}", s),
						}
					}
					&Class | &SystemClass | &PrimitiveArray | &StringArray => {
						// ignore additional_info
						parse_class_vec.push(box(&parse_MemberReferenceRecord_or_ObjectNullRecord));
					}
					&String => {
						parse_class_vec.push(box(&parse_String));
					}
					s @ _ => panic!("{:?}", s),
				}
			}
			parse_class_vec
		};

		let mut parse_class_map = HashMap::new();
		let mut json_vec = vec![];
		for id_and_values in id_and_values_vec.iter() {
			let &(class_id, ref values) = id_and_values;
			let (class_name, member_names, binary_types, additional_infos) = get_class_info(class_id);
			let parse_class_vec = parse_class_map.entry(class_id).or_insert_with(|| {
				create_parse_class_vec(binary_types, additional_infos)
			});

			let mut map = Map::new();
			for (name, value, parse_class) in Zip::new((member_names, values, parse_class_vec)) {
				map.insert(name.clone(), parse_class(value));
			}

			json_vec.push({
				let mut m = Map::new();
				m.insert(class_name, JValue::Object(map));
				JValue::Object(m)
			});
		}

		let json_path = parent_dir.join(Path::new(filename).with_extension("json"));
		let mut json_file = OpenOptions::new().write(true).create(true).truncate(true).open(json_path).unwrap();
		if to_writer_pretty(&mut json_file, &JValue::Array(json_vec)).is_err() {
			panic!("Error while writing json file");
		}
	}
}
