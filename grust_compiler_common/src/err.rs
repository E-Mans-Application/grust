//! Error-handling.

prelude! {}

/// A [`Loc`]ation and a `T`-value.
#[derive(Debug)]
pub struct LocAnd<T> {
    /// The location ([`Span`]).
    loc: Option<Loc>,
    /// Some value.
    val: T,
}
impl<T> ops::Deref for LocAnd<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
impl<T> ops::DerefMut for LocAnd<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}
impl<T> LocAnd<T> {
    pub fn loc(&self) -> Option<Loc> {
        self.loc
    }
    pub fn get(&self) -> &T {
        &self.val
    }
}

/// Note kind, refines the notion of [`Note`].
#[derive(Debug)]
pub enum NoteKind {
    /// A plain message.
    Msg { msg: String },
}
impl Display for NoteKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msg { msg } => msg.fmt(f),
        }
    }
}
impl From<String> for NoteKind {
    fn from(msg: String) -> Self {
        Self::Msg { msg }
    }
}
impl From<&'_ str> for NoteKind {
    fn from(msg: &str) -> Self {
        Self::Msg { msg: msg.into() }
    }
}

/// Adds information to errors.
pub type Note = LocAnd<NoteKind>;

impl Note {
    pub fn new(loc: Option<Loc>, kind: impl Into<NoteKind>) -> Self {
        Self {
            loc,
            val: kind.into(),
        }
    }
    pub fn new_at(loc: Loc, kind: impl Into<NoteKind>) -> Self {
        Self::new(Some(loc), kind)
    }
    pub fn new_locless(kind: impl Into<NoteKind>) -> Self {
        Self::new(None, kind)
    }

    pub fn msg(loc: Loc, msg: impl Into<String>) -> Self {
        Self::new_at(loc, msg.into())
    }
}

/// Error kind, refines the notion of [`Error`].
#[derive(Debug)]
pub enum ErrorKind {
    /// A plain error message.
    Msg { msg: String },
    /// Encountering an unknown element.
    UnknownIdent {
        /// The unknown identifier.
        name: String,
    },
    /// Unknown initialization.
    UnknownInit {
        /// The unknown initialization.
        name: String,
    },
    /// Encountering an unknown flow.
    UnknownFlow {
        /// The unknown identifier.
        name: String,
    },
    /// Encountering an unknown component.
    UnknownComponent {
        /// The unknown identifier.
        name: String,
    },
    /// Encountering an unknown type.
    UnknownType {
        /// The unknown identifier.
        name: String,
    },
    /// Encountering an unknown enumeration.
    UnknownEnumeration {
        /// The unknown enumeration identifier.
        name: String,
    },
    /// Encountering an unknown enumeration.
    UnknownEnumerationElement {
        /// The unknown enumeration identifier.
        name: String,
        /// The unknown enumeration variant identifier.
        variant: String,
    },
    /// Encountering an unknown field.
    UnknownField {
        /// The structure of the supposed field.
        structure_name: String,
        /// The unknown field.
        field_name: String,
    },
    /// A field is missing.
    MissingField {
        /// The structure of the missing field.
        structure_name: String,
        /// The missing field.
        field_name: String,
    },
    /// The index is out of bounds.
    IndexOutOfBounds,
    /// Redefine an already defined element.
    AlreadyDefinedElement {
        /// The known identifier.
        name: String,
    },
    /// Multiple types for instantiation.
    AlreadyTyped {
        /// The identifier that is already typed.
        name: String,
    },
    /// Incompatible type.
    IncompatibleType {
        /// Given type.
        given_type: Typ,
        /// Expected type.
        expected_type: Typ,
    },
    /// Not reactive.
    NotReactive {
        /// Given reactive type.
        given_react_type: ReactTyp,
        /// Expected reactive type.
        expected_react_type: ReactTyp,
    },
    /// Incompatible tuple.
    IncompatibleTuple,
    /// Incompatible match statements.
    IncompatibleMatchStatements {
        /// Expected number of statements.
        expected: usize,
        /// Received number of statements.
        received: usize,
    },
    /// Missing match statement.
    MissingMatchStatement {
        /// Ident of the missing statement in match.
        identifier: String,
    },
    /// Not statement pattern error.
    NotStatementPattern,
    /// Given inputs are not of the right number.
    ArityMismatch {
        /// The given number of inputs.
        input_count: usize,
        /// The expected number of inputs.
        arity: usize,
    },
    /// Calling an unknown output identifier.
    UnknownOutputIdent {
        /// The component identifier.
        comp_name: String,
        /// The unknown identifier.
        ident: String,
    },
    /// Expected constant expression.
    ExpectConstant,
    /// Expected at least one input.
    ExpectInput,
    /// Expect a type to this declaration.
    ExpectType {
        /// The identifier that is not typed.
        name: String,
    },
    /// Expected an arithmetic type.
    ExpectArithType {
        /// Given type.
        given_type: Typ,
    },
    /// Expected lambda with input type.
    ExpectLambda {
        /// Expected types as input for the lambda.
        input_types: Vec<Typ>,
        /// Given type instead of the lambda.
        given_type: Typ,
    },
    /// Expected option type.
    ExpectOption {
        /// Given type instead of the option.
        given_type: Typ,
    },
    /// Expected structure type.
    ExpectStructure {
        /// Given type instead of the structure.
        given_type: Typ,
    },
    /// Expected tuple type.
    ExpectTuple {
        /// Given type instead of the structure.
        given_type: Typ,
    },
    /// Expected array type.
    ExpectArray {
        /// Given type instead of the array.
        given_type: Typ,
    },
    /// Expected event type.
    ExpectEvent {
        /// Given type instead of the event.
        given_type: Typ,
    },
    /// Expected signal type.
    ExpectSignal {
        /// Given type instead of the signal.
        given_type: Typ,
    },
    /// Expected option pattern.
    ExpectOptionPattern,
    /// Expected tuple pattern.
    ExpectTuplePattern,
    /// Incompatible array length.
    IncompatibleLength {
        /// Given length.
        given_length: usize,
        /// Expected length.
        expected_length: usize,
    },
    /// Can not infer type.
    NoTypeInference,
    /// Causality error.
    NotCausalIdent {
        /// Identifier.
        ident: String,
    },
    /// Causality error.
    NotCausalComponent {
        /// Component's name.
        component: String,
    },
    /// Unused import error.
    UnusedImport {
        /// Import's name.
        name: String,
    },
    /// Unused identifier error.
    UnusedIdent {
        /// Component's name.
        component: String,
        /// Identifier.
        ident: String,
    },
}
mk_new! { impl ErrorKind =>
    Msg: msg { msg: impl Into<String> = msg.into() }
    ArityMismatch: arity_mismatch {
        input_count: usize, arity: usize,
    }
    IndexOutOfBounds: oob {}
    MissingMatchStatement: missing_match_stmt {
        identifier: impl Into<String> = identifier.into(),
    }
    MissingField: missing_field {
        structure_name: impl Into<String> = structure_name.into(),
        field_name: impl Into<String> = field_name.into(),
    }

    IncompatibleType: incompatible_types {
        given_type: Typ, expected_type: Typ,
    }
    IncompatibleLength: incompatible_length {
        given_length: usize,
        expected_length: usize
    }
    IncompatibleMatchStatements: incompatible_match {
        expected: usize,
        received: usize,
    }
    IncompatibleTuple: incompatible_tuple {}

    ExpectArithType: expected_arith_type {
        given_type: Typ,
    }
    ExpectLambda: expected_lambda {
        input_types: Vec<Typ>, given_type: Typ,
    }
    ExpectEvent: expected_event {
        given_type: Typ
    }
    ExpectSignal: expected_signal {
        given_type: Typ
    }
    ExpectStructure: expected_structure {
        given_type: Typ
    }
    ExpectArray: expected_array {
        given_type: Typ
    }
    ExpectTuple: expected_tuple {
        given_type: Typ
    }
    ExpectInput: expected_input {}
    ExpectTuplePattern: expected_tuple_pat {}
    ExpectOptionPattern: expected_option_pat {}
    ExpectConstant: expected_constant {}
    ExpectType: expected_ty {
        name: impl Into<String> = name.into(),
    }

    NotReactive: not_reactive {
        given_react_type: ReactTyp, expected_react_type: ReactTyp,
    }

    AlreadyDefinedElement: elm_redef {
        name: impl Into<String> = name.into(),
    }
    AlreadyTyped: re_ty {
        name: impl Into<String> = name.into(),
    }

    UnknownIdent: unknown_ident {
        name: impl Into<String> = name.into(),
    }
    UnknownInit: unknown_init {
        name: impl Into<String> = name.into(),
    }
    UnknownEnumerationElement: unknown_enum_elem {
        name: impl Into<String> = name.into(),
        variant: impl Into<String> = variant.into(),
    }
    UnknownFlow: unknown_flow {
        name: impl Into<String> = name.into(),
    }
    UnknownComponent: unknown_comp {
        name: impl Into<String> = name.into(),
    }
    UnknownType: unknown_type {
        name: impl Into<String> = name.into(),
    }
    UnknownField: unknown_field {
        structure_name: impl Into<String> = structure_name.into(),
        field_name: impl Into<String> = field_name.into(),
    }

    UnusedImport: unused_import {
        name: impl Into<String> = name.into(),
    }
    UnusedIdent: unused_ident {
        ident: impl Into<String> = ident.into(),
        component: impl Into<String> = component.into(),
    }

    NotCausalIdent : ident_non_causal {
        ident: impl Into<String> = ident.into(),
    }
    NotCausalComponent : comp_non_causal {
        component: impl Into<String> = component.into(),
    }
}

impl From<String> for ErrorKind {
    fn from(msg: String) -> Self {
        Self::Msg { msg }
    }
}
impl From<&'_ str> for ErrorKind {
    fn from(msg: &str) -> Self {
        Self::Msg { msg: msg.into() }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorKind::*;
        match self {
            Msg { msg } => msg.fmt(f),
            UnknownIdent { name } => write!(f, "unknown identifier `{name}`"),
            UnknownInit { name } => write!(f, "identifier `{name}` not initialized"),
            UnknownEnumerationElement { name, variant } => {
                write!(f, "unknown enumeration variant `{name}::{variant}`")
            }
            UnknownFlow { name } => write!(f, "unknown flow `{name}`"),
            UnknownComponent { name } => write!(f, "unknown component `{name}`"),
            UnknownType { name } => write!(f, "unknown type `{name}`"),
            UnknownEnumeration { name } => write!(f, "unknown enumeration `{name}`"),
            UnknownField {
                structure_name,
                field_name,
                ..
            } => write!(
                f,
                "unknown field `{field_name}` in `{structure_name}` structure"
            ),
            MissingField {
                structure_name,
                field_name,
                ..
            } => write!(
                f,
                "missing field `{field_name}` in `{structure_name}` structure"
            ),
            IndexOutOfBounds => write!(f, "index out of bounds"),
            AlreadyDefinedElement { name } => {
                write!(f, "trying to redefine element `{name}`")
            }
            AlreadyTyped { name } => write!(f, "trying to re-type `{name}`"),
            IncompatibleType {
                given_type,
                expected_type,
            } => write!(
                f,
                "type mismatch: got `{given_type}`, expected `{expected_type}`"
            ),
            NotReactive {
                given_react_type,
                expected_react_type,
            } => write!(
                f,
                "not reactive: got `{given_react_type}`, expected `{expected_react_type}`"
            ),
            IncompatibleTuple => write!(f, "incompatible tuple"),
            IncompatibleMatchStatements { expected, received } => write!(
                f,
                "incompatible match statements: got {received}, expected {expected}"
            ),
            MissingMatchStatement { identifier } => {
                write!(f, "missing match statement for `{identifier}`")
            }
            NotStatementPattern => write!(f, "not statement pattern"),
            ArityMismatch { input_count, arity } => write!(
                f,
                "arity mismatch: got {} input{}, expected {}",
                input_count,
                plural(*input_count),
                arity
            ),
            UnknownOutputIdent { comp_name, ident } => write!(
                f,
                "unknown output ident in component `{comp_name}`: `{ident}`"
            ),
            ExpectType { name } => write!(f, "expected type for `{name}`"),
            ExpectConstant => write!(f, "expected constant"),
            ExpectInput => write!(f, "expected input"),
            ExpectArithType { given_type } => {
                write!(
                    f,
                    "expected a number such as `7` or `1.32`, got a value of type`{given_type}`"
                )
            }
            ExpectLambda { given_type, .. } => write!(f, "expected lambda but given {given_type}"),
            ExpectOption { given_type } => write!(f, "expected option but given {given_type}"),
            ExpectStructure { given_type } => {
                write!(f, "expected structure but given {given_type}")
            }
            ExpectTuple { given_type } => write!(f, "expected tuple but given {given_type}"),
            ExpectArray { given_type } => write!(f, "expected array but given {given_type}"),
            ExpectEvent { given_type } => write!(f, "expected event but given {given_type}"),
            ExpectSignal { given_type } => write!(f, "expected signal but given {given_type}"),
            ExpectOptionPattern => write!(f, "expected option pattern"),
            ExpectTuplePattern => write!(f, "expected tuple pattern"),
            IncompatibleLength {
                given_length,
                expected_length,
            } => write!(
                f,
                "incompatible length, given {given_length} but expect {expected_length}"
            ),
            NoTypeInference => write!(f, "no type inference"),
            NotCausalIdent { ident } => write!(f, "ident `{ident}` is not causal"),
            NotCausalComponent { component } => write!(f, "component `{component}` is not causal"),
            UnusedIdent { ident, component } => {
                write!(f, "ident `{ident}` is unused in component `{component}`")
            }
            UnusedImport { name } => {
                write!(f, "import `{name}` is unused in this program")
            }
        }
    }
}

/// A [`Loc`]ation and an [`ErrorKind`].
pub type Error = LocAnd<(ErrorKind, Vec<Note>)>;

impl Error {
    pub fn new(loc: Option<Loc>, kind: impl Into<ErrorKind>) -> Self {
        Self {
            loc,
            val: (kind.into(), vec![]),
        }
    }
    pub fn new_at(loc: Loc, kind: impl Into<ErrorKind>) -> Self {
        Self::new(Some(loc), kind)
    }
    pub fn new_locless(kind: impl Into<ErrorKind>) -> Self {
        Self::new(None, kind)
    }

    pub fn msg(loc: Loc, msg: impl Into<String>) -> Self {
        Self::new_at(loc, msg.into())
    }

    pub fn destruct(self) -> (Option<Loc>, ErrorKind, Vec<Note>) {
        let Self {
            loc,
            val: (kind, notes),
        } = self;
        (loc, kind, notes)
    }

    pub fn error(&self) -> &ErrorKind {
        &self.val.0
    }
    pub fn notes(&self) -> &Vec<Note> {
        &self.val.1
    }
    pub fn notes_mut(&mut self) -> &mut Vec<Note> {
        &mut self.val.1
    }

    pub fn add_note_mut(&mut self, note: Note) {
        self.notes_mut().push(note);
    }
    pub fn add_note(mut self, note: Note) -> Self {
        self.add_note_mut(note);
        self
    }

    // pub fn to_syn_error(&self) -> syn::Error {
    //     // we're ignoring notes, sadly...
    //     // let loc = self.loc.unwrap_or_else(Loc::mixed_site);
    //     let loc = self.loc.expect("internal error: error has no location >_<");
    //     // println!("-> {:?}", loc);
    //     let msg = self.error().to_string();
    //     let error = syn::Error::new(loc.span, msg);
    //     // println!("   {:?}", error.span());
    //     error
    // }
    // pub fn into_syn_error(self) -> syn::Error {
    //     self.to_syn_error()
    // }
    // pub fn to_compile_error(&self) -> TokenStream2 {
    //     self.to_syn_error().into_compile_error()
    // }
    // pub fn into_compile_error(self) -> TokenStream2 {
    //     self.to_compile_error()
    // }

    #[cfg(feature = "diagnostics")]
    pub fn to_diagnostic(self) -> macro1::Diagnostic {
        use macro1::*;
        // println!("error:\n{}", self.error().to_string());
        let (error_kind, notes) = self.val;
        let loc = self
            .loc
            .expect("internal error: error has no location >_<")
            .unwrap();
        let mut d = Diagnostic::spanned(&[loc] as &[Span], Level::Error, error_kind.to_string());
        for note in notes {
            let msg = note.val;
            if let Some(loc) = note.loc {
                d = d.span_note(&[loc.unwrap()] as &[Span], msg.to_string());
            } else {
                d = d.note(msg.to_string());
            }
        }
        d
    }

    #[cfg(feature = "diagnostics")]
    pub fn to_note_diagnostic(self) -> macro1::Diagnostic {
        use macro1::*;
        let (error_kind, notes) = self.val;
        let loc = self
            .loc
            .expect("internal error: error has no location >_<")
            .unwrap();
        let mut d = Diagnostic::spanned(&[loc] as &[Span], Level::Note, error_kind.to_string());
        for note in notes {
            let msg = note.val;
            if let Some(loc) = note.loc {
                d = d.span_note(&[loc.unwrap()] as &[Span], msg.to_string());
            } else {
                d = d.note(msg.to_string());
            }
        }
        d
    }

    #[cfg(feature = "diagnostics")]
    pub fn emit(self) {
        self.to_diagnostic().emit();
    }

    #[cfg(feature = "diagnostics")]
    pub fn emit_note(self) {
        self.to_note_diagnostic().emit();
    }
}

/// [`Result`]-type with [`Error`] as errors.
pub type Res<T> = Result<T, Error>;

/// Termination error, the compilation has detected an error.
#[derive(Debug)]
pub struct ErrorDetected;

/// Top-level result type.
pub type TRes<T> = Result<T, ErrorDetected>;

/// Alias for `Res<()>`.
pub type URes = Res<()>;

/// Extends [`Result`], more precisely [`Res`].
pub trait ResExt: Sized {
    /// Type of the `Ok` branch of the result.
    type Inner;
    /// Adds a note if `self` is an error.
    fn err_note(self, f: impl FnOnce() -> Note) -> Self;
    /// Drains the error, if any, into an error list.
    fn dewrap(self, errors: &mut Vec<Error>) -> TRes<Self::Inner>;
    fn move_err(self, errors: &mut Vec<Error>) {
        let _ = self.dewrap(errors);
    }
}

impl<T> ResExt for Res<T> {
    type Inner = T;
    fn err_note(mut self, f: impl FnOnce() -> Note) -> Self {
        if let Err(err) = &mut self {
            err.add_note_mut(f());
        }
        self
    }
    fn dewrap(self, errors: &mut Vec<Error>) -> TRes<T> {
        match self {
            Ok(res) => Ok(res),
            Err(e) => {
                errors.push(e);
                Err(ErrorDetected)
            }
        }
    }
}

#[macro_export]
macro_rules! mk_error {
    { @ $loc:expr => $e:expr } => {
        // compile_error!(concat!("tokens: ", stringify!($e)));
        $crate::prelude::Error::new_at($loc.into(), $e)
            .add_note($crate::note!(@ $loc => "raised at `{}:{}`", file!(), line!()))
    };
    { @ $loc:expr => $($stuff:tt)* } => {
        $crate::mk_error!(@ $loc => format!($($stuff)*))
    };
}

#[macro_export]
macro_rules! lnote {
    {$($stuff:tt)*} => { || {note!($($stuff)*)} };
}

#[macro_export]
macro_rules! lerror {
    {$($stuff:tt)*} => { || {error!($($stuff)*)} };
}

#[macro_export]
macro_rules! note {
    { @ $loc:expr => $e:expr } => {
        $crate::prelude::Note::new_at($loc.into(), $e)
    };
    { @ $loc:expr => $($stuff:tt)* } => {
        $crate::prelude::Note::new_at($loc.into(), format!($($stuff)*))
    };
    { $e:expr } => {
        $crate::prelude::Note::new(None, $e)
    };
    { $($stuff:tt)* } => {
        $crate::prelude::Note::new(None, format!($($stuff)*))
    };
}

/// Generates a grust error at some location.
///
/// # Examples
///
/// ```rust
/// # grust_compiler_common::prelude! {}
/// let loc = Loc::test_dummy();
/// let some_data = "<data>";
/// // simple error
/// let _error = error!{ @ loc =>
///     "something went wrong because of this data: `{}`", some_data
/// };
///
/// let loc1 = Loc::test_dummy();
/// let loc2 = Loc::test_dummy();
/// let data1 = "<data1>";
/// let data2 = "<data2>";
/// // error with two notes
/// let _error = error! { @ loc =>
///     "something went wrong because of this data: `{}`", some_data,
///     => | @loc1 => "probably because of this: `{}`", data1,
///     => | @loc2 => "or maybe because of that: `{}`", data2,
/// };
/// ```
#[macro_export]
macro_rules! error {
    { @ $loc:expr
        => $($error:expr),* $(,)?
        $( => | $($notes:tt)* )?
    } => {{
        #[allow(unused_mut)]
        let mut error = {
            #[allow(unused_imports)]
            use $crate::prelude::ErrorKind::*;
            $crate::mk_error!(@ $loc => $($error),*)
        };
        $({
            #[allow(unused_imports)]
            use $crate::prelude::NoteKind::*;
                $crate::error!( (@extend &mut error) | $($notes)* );
        })?
        error
    }};
    { (@extend $error:expr)
        | @ $loc:expr => $($expr:expr),* $(,)?
        $(=> | $($tail:tt)*)?
    } => {{
        $error.add_note_mut(note!(@ $loc => $($expr),*));
        $( $crate::error!((@extend $error)  | $($tail)*) )?
    }};
    // Note parsing.
    // { (@extend $error:expr)
    //     | @ $loc:expr => $($expr:expr),* ,
    // } => {
    //     $error.add_note_mut(note!(@ $loc => $($expr),*))
    // };
}

#[macro_export]
macro_rules! noErrorDesc {
    {} => {
        panic!("[{}:{}] no error description available for this error", file!(), line!())
    };
    {
        $($stuff:tt)*
    } => {
        panic!("[{}:{}] {}", file!(), line!(), format!($($stuff)*))
    };
}

#[macro_export]
macro_rules! bail {
    {
        when $($cnd:expr => { $($stuff:tt)* } )+
    } => {
        $(
            if $cnd {
                bail!($($stuff)*)
            }
        )*
    };

    { $($stuff:tt)* } => {
        return Err( error!($($stuff)*) )
    };
}

#[macro_export]
macro_rules! bad {
    {} => { return (Err(ErrorDetected) as TRes<_>) };
    {
        $errors:expr, when $($cnd:expr => { $($stuff:tt)* } )+
    } => {
        $(
            if $cnd {
                bad!($errors, $($stuff)*)
            }
        )*
    };
    { $errors:expr, $($stuff:tt)* } => {{
        let info = format!("[internal] error raised at `{}:{}`", file!(), line!());
        $errors.push(error!($($stuff)*).add_note(Note::new(None, info)));
        bad!()
    }};
}
