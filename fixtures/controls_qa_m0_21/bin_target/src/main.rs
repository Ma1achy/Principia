fn main() {
    println!("{}", qa_controls_bin_target::quadruple(2));
}

#[cfg(test)]
mod bin_tests {
    #[test]
    fn quadruples() {
        qa_controls_bin_target::check_quadruple(qa_controls_bin_target::quadruple);
    }
}
