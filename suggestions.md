**Is your feature request related to a problem? Please describe.**
Hi, from some Googling I see that the static lifetime requirement on the callbacks has come up multiple times in the past. In every discussion, using Rc<RefCell<T>> is mentioned as the solution. I understand that it's not a performance problem, but for me it's giving up all of the reasons (ergonomic == correct) that I chose to use Rust. I'd be opened up to [memory leaks](https://github.com/rust-lang/rust/issues/15572#issuecomment-48592490), runtime failures (BorrowMutError), and the general need-to-play-with-it-to-be-sure testing that I'd like to avoid. I actually went and read up to the unsafe portion of WayTooManyLinkedLists, to be sure that I understood RefCell/Rc more than just the necessary boilerplate to follow the examples, and to make sure that ref-counting wasn't truly necessary for most Rust apps, because IMO this is a regression from compile-time-known-lifetimes. 

But having read all of that, I think it's unnecessary in this case, and I thought moving my state into Cursive's event loop was a possible solution. letting it pass back my mutable state into my closures. I see your objection to parameterizing Cursive on lifetime; I was going to do the refactor anyway just to see how it worked, but then I found #262 adding the perfect bypass. I just need to figure out how to use it.  Here are some examples:

This fails to compile because need to borrow siv as mutable in order to call set_content, and I can't move it into the callback for obvious reasons.
```
            /*
            siv.with_user_data(|app: &mut App| {
                // Failed experiment, give up and accept rcs for now :'[
                app.next_line();
                siv.call_on_name(text, |t: &mut TextView| { t.set_content(app.get_view()); });
            });
            */
```

Also, my App object which I hold an `Rc<RefCell<>>` to, must not contain any references, must own everything.


**Describe the solution you'd like**
A clear and concise description of what you want to happen.

**Describe alternatives you've considered**
A clear and concise description of any alternative solutions or features you've considered.

**Additional context**
Add any other context or screenshots about the feature request here.

https://github.com/gyscos/cursive/blob/main/cursive/examples/advanced_user_data.rs