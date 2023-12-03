use std::collections::HashMap;

#[derive(Default)]
pub struct Blackboard(HashMap<String, Box<dyn std::any::Any>>);

impl Blackboard {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn write<T>(&mut self, key: String, data: T)
    where
        T: 'static,
    {
        self.0.insert(key, Box::new(data));
    }

    pub fn read_ref<T>(&self, key: &String) -> Option<&T>
    where
        T: 'static,
    {
        match self.0.get(key) {
            Some(data) => data.downcast_ref(),
            None => None,
        }
    }

    pub fn read_ref_mut<T>(&mut self, key: &String) -> Option<&mut T>
    where
        T: 'static,
    {
        match self.0.get_mut(key) {
            Some(data) => data.downcast_mut(),
            None => None,
        }
    }

    pub fn read<T>(&self, key: &String) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.read_ref(key).cloned()
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Input<T> {
    Literal(T),
    Blackboard(String),
}

impl<T> Clone for Input<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Literal(data) => Self::Literal(data.clone()),
            Self::Blackboard(key) => Self::Blackboard(key.clone()),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Output {
    Blackboard(String),
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    #[derive(Debug)]
    struct Tester {
        map: HashMap<usize, Vec<String>>,
    }

    impl Tester {
        pub fn new() -> Self {
            Self {
                map: HashMap::new(),
            }
        }
        pub fn fill_random(&mut self, bound: usize) {
            for i in 0..bound {
                let collected_vec: Vec<String> =
                    (i..bound).into_iter().map(|d| d.to_string()).collect();
                self.map.insert(i, collected_vec);
            }
        }
    }

    #[test]
    fn test_blackboard_read_and_write_good() {
        let mut blackboard = Blackboard::new();

        // Floats
        let float32: f32 = 123.45;
        blackboard.write("float32".into(), float32);
        let float32 = blackboard.read::<f32>(&"float32".into());
        assert!(float32.is_some());
        assert_eq!(float32.unwrap(), 123.45);

        let float64: f64 = 123.4567890;
        blackboard.write("float64".into(), float64);
        let float64 = blackboard.read::<f64>(&"float64".into());
        assert!(float64.is_some());
        assert_eq!(float64.unwrap(), 123.4567890);

        // Tuples
        let tuple = (12, "test", 34.56);
        blackboard.write("tuple".into(), tuple);
        let tuple = blackboard.read::<(i32, &str, f64)>(&"tuple".into());
        assert!(tuple.is_some());
        assert_eq!(tuple.unwrap(), (12, "test", 34.56));

        // Structs
        let tester = Tester::new();
        assert_eq!(tester.map.keys().len(), 0);
        blackboard.write("tester".into(), tester);

        // NOTE, As per design, since Tester is not clone we cannot use the `read` API and are forced to use the `read_ref` API
        // This makes it so that there is no unnecessary expensive clones taking place unless explicitly required

        // We also read by ref mut and modify this tester, when we re-read from blackboard the internal values should be automatically updated!
        let tester = blackboard.read_ref_mut::<Tester>(&"tester".into());
        assert!(tester.is_some());
        let tester = tester.unwrap();
        tester.fill_random(20);
        println!("Tester: {:?}", tester);

        // Re-reading here
        let tester = blackboard.read_ref::<Tester>(&"tester".into());
        assert!(tester.is_some());
        let tester = tester.unwrap();
        assert_eq!(tester.map.keys().len(), 20);

        // Reference counted and Box types
        let boxed: Box<u32> = Box::new(10);
        blackboard.write("boxed_u32".into(), boxed);
        let boxed = blackboard.read::<Box<u32>>(&"boxed_u32".into());
        assert!(boxed.is_some());
        assert_eq!(*boxed.unwrap(), 10);

        let rc = Rc::new(10);
        blackboard.write("rc_u32".into(), rc);
        let rc = blackboard.read::<Rc<i32>>(&"rc_u32".into());
        assert!(rc.is_some());
        assert_eq!(*rc.unwrap(), 10);
    }

    #[test]
    fn test_blackboard_bad_read() {
        let mut blackboard = Blackboard::new();

        // * Value does not exist
        let read_string = blackboard.read::<String>(&"string".into());
        assert_eq!(read_string, None);

        let read_string = blackboard.read_ref_mut::<String>(&"string".into());
        assert_eq!(read_string, None);

        let string = "String".to_owned();
        blackboard.write("string".into(), string);

        // * The type is incorrect which makes us give a bad value
        let read_usize = blackboard.read::<usize>(&"string".into());
        assert_eq!(read_usize, None);

        let read_usize = blackboard.read_ref_mut::<usize>(&"string".into());
        assert_eq!(read_usize, None);
    }

    #[test]
    fn test_blackboard_write_overwrite() {
        let mut blackboard = Blackboard::new();

        let string = "Test".to_owned();
        blackboard.write("string".into(), string);
        let string = blackboard.read::<String>(&"string".into()).unwrap();
        assert_eq!(string, "Test".to_owned());

        // Overwrite the same location with a different data and different type
        let number = 123.34;
        blackboard.write("string".into(), number);
        let number = blackboard.read::<f64>(&"string".into()).unwrap();
        assert_eq!(number, 123.34);
    }
}
