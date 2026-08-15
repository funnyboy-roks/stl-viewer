use thiserror::Error;

type Vec3 = (f32, f32, f32);

#[derive(Debug, Clone)]
pub struct Facet {
    pub normal: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub v3: Vec3,
}

#[derive(Debug, Clone)]
pub struct Solid {
    pub name: Option<String>,
    pub facets: Vec<Facet>,
}

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("Expected {}, get {:?}", expected, actual)]
    ExpectedWord {
        expected: String,
        actual: Option<String>,
    },
    #[error("Expected f32, got {:?}", actual)]
    ExpectedF32 { actual: Option<String> },
}

#[derive(Debug, Clone)]
pub struct StrParser<'a> {
    content: &'a str,
    offset: usize,
}

impl<'a> StrParser<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content, offset: 0 }
    }

    pub fn take_word(&mut self) -> Option<&str> {
        self.offset +=
            self.content[self.offset..].len() - self.content[self.offset..].trim_start().len();
        let word = self.content[self.offset..].split_whitespace().next()?;
        self.offset += word.len();
        Some(word)
    }

    pub fn expect_word(&mut self, word: &'_ str) -> Result<(), ParseError> {
        let start = self.offset;
        let actual = self.take_word().ok_or_else(|| ParseError::ExpectedWord {
            expected: word.into(),
            actual: None,
        })?;
        if actual == word {
            Ok(())
        } else {
            let actual = actual.into();
            self.offset = start; // untake word
            Err(ParseError::ExpectedWord {
                expected: word.into(),
                actual: Some(actual),
            })
        }
    }

    pub fn expect_f32(&mut self) -> Result<f32, ParseError> {
        let word = self
            .take_word()
            .ok_or(ParseError::ExpectedF32 { actual: None })?;
        word.parse().map_err(|_| ParseError::ExpectedF32 {
            actual: Some(word.into()),
        })
    }

    /// ```stl
    /// vertex 0 19.809 0.412201
    /// ```
    pub fn take_vertex(&mut self) -> Result<(f32, f32, f32), ParseError> {
        self.expect_word("vertex")?;

        let x = self.expect_f32()?;
        let y = self.expect_f32()?;
        let z = self.expect_f32()?;

        Ok((x, y, z))
    }

    /// ```stl
    /// outer loop
    ///   vertex 0 19.809 0.412201
    ///   vertex 0 19.809 1.58778
    ///   vertex 0 20 1
    /// endloop
    /// ```
    pub fn take_oloop(&mut self) -> Result<(Vec3, Vec3, Vec3), ParseError> {
        self.expect_word("outer")?;
        self.expect_word("loop")?;

        let v1 = self.take_vertex()?;
        let v2 = self.take_vertex()?;
        let v3 = self.take_vertex()?;

        self.expect_word("endloop")?;

        Ok((v1, v2, v3))
    }

    /// ```stl
    /// facet normal -1 0 0
    ///   outer loop
    ///     vertex 0 19.809 0.412201
    ///     vertex 0 19.809 1.58778
    ///     vertex 0 20 1
    ///   endloop
    /// endfacet
    /// ```
    pub fn take_facet(&mut self) -> Result<Facet, ParseError> {
        self.expect_word("facet")?;
        self.expect_word("normal")?;

        let x = self.expect_f32()?;
        let y = self.expect_f32()?;
        let z = self.expect_f32()?;

        let (v1, v2, v3) = self.take_oloop()?;

        self.expect_word("endfacet")?;

        Ok(Facet {
            normal: (x, y, z),
            v1,
            v2,
            v3,
        })
    }

    pub fn take_solid(&mut self) -> Result<Solid, ParseError> {
        self.expect_word("solid")?;
        let name = self.take_word().map(Into::into);

        let mut facets = Vec::new();

        loop {
            let offset = self.offset;
            let word = self.take_word();
            if word != Some("facet") {
                self.offset = offset;
                break;
            }
            self.offset = offset;

            facets.push(self.take_facet()?);
        }

        self.expect_word("endsolid")?;
        let name2 = self.take_word();

        assert_eq!(name.as_deref(), name2);

        Ok(Solid { name, facets })
    }
}

fn parse_bin_vec(content: &[u8]) -> (Vec3, &[u8]) {
    let (chunks, _) = content.as_chunks();
    let x = f32::from_le_bytes(chunks[0]);
    let y = f32::from_le_bytes(chunks[1]);
    let z = f32::from_le_bytes(chunks[2]);
    ((x, y, z), &content[std::mem::size_of::<f32>() * 3..])
}

fn parse_bin(content: &[u8]) -> Result<Solid, ParseError> {
    let content = &content[80..];
    let (ntriangles, content) = content.split_first_chunk().unwrap();
    let mut facets = Vec::with_capacity(u32::from_le_bytes(*ntriangles) as usize);
    let mut content = content;
    for _ in 0..u32::from_le_bytes(*ntriangles) {
        let (norm, rest) = parse_bin_vec(content);
        let (v1, rest) = parse_bin_vec(rest);
        let (v2, rest) = parse_bin_vec(rest);
        let (v3, rest) = parse_bin_vec(rest);
        facets.push(Facet {
            normal: norm,
            v1,
            v2,
            v3,
        });
        content = &rest[2..];
    }
    Ok(Solid { name: None, facets })
}

pub fn parse_stl(content: &[u8]) -> Result<Solid, ParseError> {
    if content.starts_with(b"solid") {
        StrParser::new(str::from_utf8(content).unwrap()).take_solid()
    } else {
        parse_bin(content)
    }
}

#[cfg(test)]
mod test {
    use crate::stl::StrParser;

    #[test]
    fn it_works() {
        let content = include_str!("../test.stl");

        let mut parser = StrParser::new(content);

        dbg!(parser.take_solid().unwrap());
    }
}
