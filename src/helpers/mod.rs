pub mod error;

#[derive(Clone)]
pub struct WithAttr<'a, T>(pub &'a [&'static str], pub &'static str, pub Option<T>);

impl<T: maud::Render> maud::Render for WithAttr<'_, T> {
    fn render(&self) -> maud::Markup {
        let mut buffer = String::new();
        self.render_to(&mut buffer);
        maud::PreEscaped(buffer)
    }

    fn render_to(&self, buffer: &mut String) {
        // open tag
        buffer.push('<');
        buffer.push_str(&self.1);
        buffer.push(' ');
        // render attrs
        buffer.push_str(&self.0.join(" "));
        buffer.push('>');

        let is_self_closing = match self.1 {
            "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
            | "param" | "source" | "track" | "wbr" => true,
            _ => false,
        };

        if is_self_closing {
            buffer.push_str(" />");
            if let Some(x) = &self.2 {
                x.render_to(buffer);
            }
        } else {
            if let Some(x) = &self.2 {
                x.render_to(buffer);
            }
            buffer.push_str("</");
            buffer.push_str(&self.1);
            buffer.push('>');
        }
    }
}
