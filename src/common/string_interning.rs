use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
/// The Id of an interned [`String`]. See [`StringInterner`] for more informations.
pub struct StringId(u16);

#[derive(Debug, Clone, Default)]
/// `imo`'s string interner. 
/// \
/// Contains a [`Vec`] which stores the interned strings, and a [`FxHashMap`] for mapping [`String`]s to [`StringId`]s.
/// \
/// Example : 
/// 
/// ```code
/// pub struct Context {
///     interner: StringInterner, 
///     foo: Foo,
/// }
/// 
/// impl Context {
///     pub fn new() -> Self {
///         Self { interner: StringInterner::Default(), foo: Foo::new() }
///     }
/// 
///     pub fn get_foo(&self) -> Option<String> {
///         self.interner.get(self.foo.id)
///     }
/// }
/// ```
pub struct StringInterner {
    buffer: Vec<String>,
    map: FxHashMap<String, StringId>,
}

impl StringInterner {
    /// Returns the corresponding [`StringId`] to the given [`str`].
    /// \
    /// If the string isn't interned yet, it generates a new [`StringId`], pushes the string in the buffer, and inserts the [`StringId`] in the map.
    pub fn get_or_intern(&mut self, str: &str) -> StringId {
        if let Some(id) = self.map.get(str) {
            return *id;
        }

        let new_id = StringId(self.buffer.len() as u16);
        self.buffer.push(str.to_string());
        self.map.insert(str.to_string(), new_id);

        new_id
    }


    /// Returns the [`String`] of given id. If the [`String`] doesn't exists (if the index is out of bounds), it returns [`None`]
    pub fn get_string(&self, id: StringId) -> Option<String> {
        self.buffer.get(id.0 as usize).cloned()
    }

    /// Returns the [`String`] of given id, mapped as an `&str`. If the [`String`] doesn't exists (if the index is out of bounds), it returns [`None`].
    pub fn get_str(&self, id: StringId) -> Option<&str> {
        self.buffer.get(id.0 as usize).map(|s| s.as_str())
    }
}