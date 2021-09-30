use super::*;

pub struct Stream<'a, I, S: Span, Iter: Iterator<Item = (I, S)> + ?Sized = dyn Iterator<Item = (I, S)> + 'a> {
    pub(crate) phantom: PhantomData<&'a ()>,
    pub(crate) ctx: S::Context,
    pub(crate) offset: usize,
    pub(crate) buffer: Vec<Option<(I, S)>>,
    pub(crate) iter: Iter,
}

impl<'a, I: Clone, S: Span> Stream<'a, I, S> {
    pub(crate) fn offset(&self) -> usize { self.offset }

    pub(crate) fn save(&self) -> usize { self.offset }
    pub(crate) fn revert(&mut self, offset: usize) { self.offset = offset; }

    fn pull_until(&mut self, offset: usize) -> &Option<(I, S)> {
        while self.buffer.len() <= offset {
            self.buffer.push(self.iter.next());
        }
        &self.buffer[offset]
    }

    pub(crate) fn next(&mut self) -> (usize, S, Option<I>) {
        match self.pull_until(self.offset).clone() {
            Some((out, span)) => {
                self.offset += 1;
                (self.offset - 1, span, Some(out))
            },
            None => (self.offset, S::new(self.ctx.clone(), None..None), None),
        }
    }

    pub(crate) fn zero_span(&mut self) -> S {
        let start = self.pull_until(self.offset.saturating_sub(1)).as_ref().and_then(|(_, s)| s.end());
        let end = self.pull_until(self.offset).as_ref().and_then(|(_, s)| s.start());
        S::new(self.ctx.clone(), start..end)
    }

    pub(crate) fn attempt<R, F: FnOnce(&mut Self) -> (bool, R)>(&mut self, f: F) -> R {
        let old_offset = self.offset;
        let (commit, out) = f(self);
        if !commit {
            self.offset = old_offset;
        }
        out
    }

    pub(crate) fn try_parse<O, E, F: FnOnce(&mut Self) -> PResult<O, E>>(&mut self, f: F) -> PResult<O, E> {
        self.attempt(move |stream| {
            let out = f(stream);
            (out.1.is_ok(), out)
        })
    }
}

impl<'a> From<&'a str> for Stream<'a, char, Range<Option<usize>>, Box<dyn Iterator<Item = (char, Range<Option<usize>>)> + 'a>> {
    fn from(s: &'a str) -> Self {
        Stream {
            phantom: PhantomData,
            ctx: (),
            offset: 0,
            buffer: Vec::new(),
            iter: Box::new(s.chars().enumerate().map(|(i, c)| (c, Some(i)..Some(i + 1)))),
        }
    }
}

impl<'a, T: Clone> From<&'a [T]> for Stream<'a, T, Range<Option<usize>>, Box<dyn Iterator<Item = (T, Range<Option<usize>>)> + 'a>> {
    fn from(s: &'a [T]) -> Self {
        Stream {
            phantom: PhantomData,
            ctx: (),
            offset: 0,
            buffer: Vec::new(),
            iter: Box::new(s.iter().cloned().enumerate().map(|(i, x)| (x, Some(i)..Some(i + 1)))),
        }
    }
}

impl<'a, T: Clone, S: Clone + Span<Context = ()>> From<&'a [(T, S)]> for Stream<'a, T, S, Box<dyn Iterator<Item = (T, S)> + 'a>> {
    fn from(s: &'a [(T, S)]) -> Self {
        Stream {
            phantom: PhantomData,
            ctx: (),
            offset: 0,
            buffer: Vec::new(),
            iter: Box::new(s.iter().cloned()),
        }
    }
}
