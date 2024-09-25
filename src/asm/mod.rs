use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;
use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
    Parser, Token,
};
use pest_derive::Parser;

use crate::common::{Error, Instruction};

#[derive(Parser)]
#[grammar = "asm/mips.pest"]
pub struct MIPSParser;

pub enum Grammar {
    /// op rt, expr(rs)
    LoadStoreOff,
    /// op rt, rs, absexpr
    ArithImm3,
    /// op rt, absexpr
    ArithImm2,
    /// op rd, rs, rt
    ArithReg,
    /// op rs, rt
    DivMult,
    /// op rs
    ArithMove,
    /// op rd, rt, shamt
    Shift,
    /// op rd, rt, rs
    ShiftVar,
    /// op addr
    Jump,
    /// op rs
    JumpRegister,
    /// op rs, rd
    JumpRegister2,
    /// op rs, rt, target
    BranchCmp,
    /// op rs, target
    BranchCmpZero,
    /// op
    None,

    /// op rt, addr
    PLoadStoreAddr,
    /// op rs, target
    PBranchCmpZero,
    /// op rd, absexpr
    PLoadStoreAbs,
    /// op rd, addr
    PLoadStoreRel,
    /// op rd, rs
    PLoadStoreReg,
    /// op rd, rt, rs
    PArithReg3,
    /// op rd, rs
    PArithReg2,
    /// op rd,
    PArithReg1,
    /// op rd, rs, absexpr
    PArithImm3,
    /// op rd, absexpr
    PArithImm2,
    /// op target
    PBranch1,
    /// op rs, target
    PBranch2,
    /// op rs, rt, target
    PBranch3Reg,
    /// op rs, absexpr, target
    PBranch3Abs,
}

lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        PrattParser::new()
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::and, Assoc::Left)
                | Op::infix(Rule::or, Assoc::Left)
                | Op::infix(Rule::xor, Assoc::Left)
                | Op::infix(Rule::sll, Assoc::Left)
                | Op::infix(Rule::srl, Assoc::Left)
                | Op::infix(Rule::sra, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left)
                | Op::infix(Rule::div, Assoc::Left)
                | Op::infix(Rule::r#mod, Assoc::Left))
            .op(Op::prefix(Rule::not))
            .op(Op::prefix(Rule::pos))
            .op(Op::prefix(Rule::neg))
    };

    /// Map from instruction to grammar. Vec allows instructions to have
    /// multiple grammars.
    pub static ref GRAMMAR_MAP: HashMap<&'static str, Vec<Grammar>> = {
        HashMap::from([
            ("jr", vec![Grammar::JumpRegister]),
            ("jalr", vec![Grammar::JumpRegister, Grammar::JumpRegister2]),
            ("jal", vec![Grammar::Jump]),
            ("j", vec![Grammar::Jump]),
            ("beq", vec![Grammar::BranchCmp]),
            ("bne", vec![Grammar::BranchCmp]),
            ("blez", vec![Grammar::BranchCmpZero]),
            ("bgtz", vec![Grammar::BranchCmpZero]),
            ("addiu", vec![Grammar::ArithImm3]),
            ("addi", vec![Grammar::ArithImm3]),
            ("sltiu", vec![Grammar::ArithImm2]),
            ("slti", vec![Grammar::ArithImm2]),
            ("andi", vec![Grammar::ArithImm3]),
            ("ori", vec![Grammar::ArithImm3]),
            ("xori", vec![Grammar::ArithImm3]),
            ("lui", vec![Grammar::ArithImm2]),
            ("lbu", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lb", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lhu", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lh", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lwl", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lwr", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("lw", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("sb", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("sh", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("swl", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("swr", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("sw", vec![Grammar::LoadStoreOff, Grammar::PLoadStoreAddr]),
            ("sllv", vec![Grammar::ShiftVar]),
            ("srlv", vec![Grammar::ShiftVar]),
            ("srav", vec![Grammar::ShiftVar]),
            ("sll", vec![Grammar::Shift]),
            ("srl", vec![Grammar::Shift]),
            ("sra", vec![Grammar::Shift]),
            ("syscall", vec![Grammar::None]),
            ("break", vec![Grammar::None]),
            ("mfhi", vec![Grammar::ArithMove]),
            ("mthi", vec![Grammar::ArithMove]),
            ("mflo", vec![Grammar::ArithMove]),
            ("mtlo", vec![Grammar::ArithMove]),
            ("multu", vec![Grammar::DivMult]),
            ("mult", vec![Grammar::DivMult]),
            ("divu", vec![Grammar::DivMult, Grammar::PArithReg3, Grammar::PArithImm3]),
            ("div", vec![Grammar::DivMult, Grammar::PArithReg3, Grammar::PArithImm3]),
            ("addu", vec![Grammar::ArithReg]),
            ("add", vec![Grammar::ArithReg]),
            ("subu", vec![Grammar::ArithReg]),
            ("sub", vec![Grammar::ArithReg]),
            ("and", vec![Grammar::ArithReg]),
            ("or", vec![Grammar::ArithReg]),
            ("nor", vec![Grammar::ArithReg]),
            ("sltu", vec![Grammar::ArithReg]),
            ("slt", vec![Grammar::ArithReg]),
            ("bltzal", vec![Grammar::BranchCmpZero]),
            ("bgezal", vec![Grammar::BranchCmpZero]),
            ("bltz", vec![Grammar::BranchCmpZero]),
            ("bgez", vec![Grammar::BranchCmpZero]),

            ("beqz", vec![Grammar::PBranchCmpZero]),
            ("bnez", vec![Grammar::PBranchCmpZero]),
            ("li", vec![Grammar::PLoadStoreAbs]),
            ("la", vec![Grammar::PLoadStoreRel]),
            ("lea", vec![Grammar::PLoadStoreRel]),
            ("move", vec![Grammar::PLoadStoreReg]),
            ("abs", vec![Grammar::PArithReg1, Grammar::PArithReg2]),
            ("neg", vec![Grammar::PArithReg1, Grammar::PArithReg2]),
            ("negu", vec![Grammar::PArithReg1, Grammar::PArithReg2]),
            ("not", vec![Grammar::PArithReg1, Grammar::PArithReg2]),
            ("rem", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("remu", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("rol", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("ror", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("mul", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("mulo", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("mulou", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("seq", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sge", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sgeu", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sgt", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sgtu", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sle", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sleu", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("sne", vec![Grammar::PArithReg2, Grammar::PArithReg3, Grammar::PArithImm2, Grammar::PArithImm2]),
            ("div", vec![Grammar::PArithReg3, Grammar::PArithImm3]),
            ("b", vec![Grammar::PBranch1]),
            ("bal", vec![Grammar::PBranch1]),
            ("beqz", vec![Grammar::PBranch2]),
            ("bnez", vec![Grammar::PBranch2]),
            ("bge", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("bgeu", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("bgt", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("bgtu", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("ble", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("bleu", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("blt", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
            ("bltu", vec![Grammar::PBranch3Reg, Grammar::PBranch3Abs]),
        ])
    };
}

pub fn dbg_parse(input: String, rule: Rule) -> Result<(), pest::error::Error<Rule>> {
    let mut tokens = MIPSParser::parse(rule, &input)?.tokens().peekable();
    let mut depth = 0;
    let chars = input.chars().collect::<Vec<_>>();
    while let Some(tok) = tokens.next() {
        if let Some(tok2) = tokens.peek() {
            match (tok, tok2) {
                (
                    Token::Start { rule, pos: start },
                    Token::End {
                        rule: rule2,
                        pos: end,
                    },
                ) => {
                    println!(
                        "{: >width$}{:?} [{}]",
                        "",
                        rule,
                        chars[start.pos()..end.pos()]
                            .iter()
                            .collect::<String>()
                            .trim(),
                        width = (depth * 2)
                    );
                    depth += 1;
                }
                (Token::Start { rule, .. }, _) => {
                    println!("{: >width$}{:?}", "", rule, width = (depth * 2));
                    depth += 1;
                }
                _ => {
                    depth -= 1;
                }
            }
        }
    }
    Ok(())
}

pub fn dbg_parse_2(input: String) -> Result<(), pest::error::Error<Rule>> {
    for l in input.lines() {
        let mut l = l.to_string();
        l.push('\n');
        dbg_parse(l, Rule::line)?
    }

    Ok(())
}
