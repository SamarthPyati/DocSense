#[derive(Debug)]
pub struct Lexer<'a> {
    // Lifetimes implemented as content is not owned
    // as used it will be assigned or shifted
    content: &'a [char]
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        // Get rid of trailing whitespace
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            // Skip the current char and assign to next
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        /* Return a slice of n len */
        let token = &self.content[0..n]; 
        self.content = &self.content[n..];
        token
    }

    fn chop_while<P>(&mut self, mut predicate: P) -> &'a [char]
    where
        P: FnMut(&char) -> bool,
    {
        /* Return a chopped slice of n length on predicate being true */
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        return self.chop(n);
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            // Ignore single digit number 
            let result = self.chop_while(|x| x.is_numeric());
            if result.len() == 1 { return None; }
            return Some(result.iter().collect());
        }

        if self.content[0].is_alphabetic() {
            let result = self.chop_while(|x| x.is_alphanumeric());
            return Some(result.iter().map(|x| x.to_ascii_uppercase()).collect());
        }
        
        // // Ignore single-character unwanted punctuation
        // let unwanted_symbols  = &[',', ';', '*', '/', '?', '{', '}', '(', ')', '.', '$', '_', '-'];
        
        // if unwanted_symbols.contains(&self.content[0]) {
        //     self.chop(1);   // skip this token 
        //     return self.next_token();     // recursively fetch next token 
        // }

        let token = self.chop(1);
        return Some(token.iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> { 
        self.next_token()
    }
}