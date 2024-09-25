.data

# the extra byte will be important later
board:  
        .space 901

scratch_board:
        .space 901

.align 2

config:
        .word 0, 0
# 0 - board size
# 4 - generations

error_size:
        .asciiz "\nWARNING: illegal board size, try again: "

error_gens:
        .asciiz "\nWARNING: illegal number of generations, try again: "        

error_cells:
        .asciiz "\nWARNING: illegal number of live cells, try again: "

error_loc:
        .asciiz "\nERROR: illegal point location\n"

colony_border:
        .asciiz "**********************\n"

colony_header:
        .asciiz "****    Colony    ****\n"

msg_board_size:
        .asciiz "\nEnter board size: "

msg_gen_count:
        .asciiz "\nEnter number of generations to run: "

msg_live_count_a:
        .asciiz "\nEnter number of live cells for colony A: "

msg_live_count_b:
        .asciiz "\nEnter number of live cells for colony B: "

msg_loc:
        .asciiz "\nStart entering locations\n"

gen_header_start:
        .asciiz "====    GENERATION "

gen_header_end:
        .asciiz "    ====\n"

board_edge:
        .asciiz "------------------------------"

dr:
        .byte   -1, -1, -1, 0, 0, 1, 1, 1

dc:
        .byte   -1, 0, 1, -1, 1, -1, 0, 1

# Bounds
MIN_BOARD = 4
MAX_BOARD = 30
MIN_GENS = 0
MAX_GENS = 20

# Frame sizes
BIG_FRAME = 56  # Store ra, s, a registers
FULL_FRAME = 40 # Store ra, s registers
SMALL_FRAME = 8 # Store ra

# Syscall codes
PRINT_INT = 1
PRINT_STRING = 4
READ_INT = 5
PRINT_CHAR = 11
EXIT2 = 17

# ASCII character values
NEWLINE = 10
SPACE = 32
PLUS = 43
COLON = 58
A = 65
B = 66
PIPE = 124

.text

.align 2

.globl  main

main:
        addi    $sp, $sp, -SMALL_FRAME
        sw      $ra, 0($sp)
# clear board & scratch_board
        li      $t0, 0
        la      $t1, board
        li      $t2, SPACE
_board1_loop:
        slti    $t9, $t0, 900
        beq     $t9, $zero, _board1_loop_exit
        add     $t3, $t1, $t0 
        sb      $t2, 0($t3)
        addi    $t0, $t0, 1
        j _board1_loop
_board1_loop_exit:
        li      $t0, 0
        la      $t1, scratch_board
        li      $t2, SPACE
_board2_loop:
        slti    $t9, $t0, 900
        beq     $t9, $zero, _board2_loop_exit
        add     $t3, $t1, $t0
        sb      $t2, 0($t3)
        addi    $t0, $t0, 1
        j _board2_loop
_board2_loop_exit:

# read config & first gen
        jal     init
        
        la      $s0, config
        lw      $s1, 4($s0)
        lw      $s0, 0($s0)

        addi    $s1, $s1, 1

# add new null terminator to board edge to trim to correct length
        la      $t0, board_edge
        add     $t0, $t0, $s0
        sb      $zero, 0($t0)


        li      $s2, 0
_generation_loop:
        beq     $s1, $s2, _generation_end
        la      $a0, board
        move    $a1, $s0
        move    $a2, $s2
        jal     print_board
        jal     next_generation
        addi    $s2, $s2, 1
        j       _generation_loop
_generation_end:
        
        lw      $ra, 0($sp)
        addi    $sp, $sp, SMALL_FRAME
        jr      $ra

#####
#
# Initializes global state by reading configuration from stdin
# Takes no arguments and returns nothing, but the board and config
# labels will be initialized.
#
#####
init:
        addi    $sp, $sp, -SMALL_FRAME
        sw      $ra, 0($sp)

# Print Banner header
        li      $v0, PRINT_CHAR
        li      $a0, NEWLINE
        syscall
        li      $v0, PRINT_STRING
        la      $a0, colony_border
        syscall
        li      $v0, PRINT_STRING
        la      $a0, colony_header
        syscall
        li      $v0, PRINT_STRING
        la      $a0, colony_border
        syscall

# read board size
        la      $a0, msg_board_size
        la      $a1, error_size
        li      $a2, MIN_BOARD
        li      $a3, MAX_BOARD
        jal     read_value
        la      $t0, config
        sw      $v0, 0($t0)

# read generation count
        la      $a0, msg_gen_count
        la      $a1, error_gens
        li      $a2, MIN_GENS
        li      $a3, MAX_GENS
        jal     read_value
        la      $t0, config
        sw      $v0, 4($t0)

# read location count
        la      $a0, msg_live_count_a
        la      $a1, error_cells
        li      $a2, 0
        la      $t0, config
        lw      $t0, 0($t0)
        mul     $a3, $t0, $t0
        jal read_value

        
# read cells for A
        move    $a0, $v0 
        li      $a1, A
        la      $a2, msg_loc
        la      $a3, error_loc
        jal     read_cells

# read b location count
        la      $a0, msg_live_count_b
        la      $a1, error_cells
        li      $a2, 0
        la      $t0, config
        lw      $t0, 0($t0)
        mul     $a3, $t0, $t0
        jal     read_value

# read cells for B
        move    $a0, $v0
        li      $a1, B
        la      $a2, msg_loc
        la      $a3, error_loc
        jal read_cells


# copy read cells to main board
        jal copy_board

        lw      $ra, 0($sp)
        addi    $sp, $sp, SMALL_FRAME
        jr      $ra



####
#
# Reads a value from stdin
# a0 contains the address of the prompt string
# a1 contains the address of the error string
# a2 contains the lower value bound
# a3 contains the upper value bound
# 
# v0 will contain the resulting int
# 
####
read_value:
        addi    $sp, $sp, -FULL_FRAME
        sw      $ra, 0($sp)
        sw      $s0, 4($sp)
        sw      $s1, 8($sp)
        sw      $s2, 12($sp)
        sw      $s3, 16($sp)
        sw      $s4, 20($sp)
        sw      $s5, 24($sp)
        sw      $s6, 28($sp)
        sw      $s7, 32($sp)

        move    $s0, $a0
        move    $s1, $a1
        move    $s2, $a2
        move    $s3, $a3

        li      $v0, PRINT_STRING
        move    $a0, $s0
        syscall
_read_value_loop:
        li      $v0, READ_INT
        syscall
        bne     $v1, $zero, _read_value_failure
        slt     $t0, $v0, $s2
        bne     $t0, $zero, _read_value_failure
        slt     $t0, $s3, $v0
        bne     $t0, $zero, _read_value_failure
        j _read_value_success
_read_value_failure:
        li      $v0, PRINT_STRING
        move    $a0, $a1
        syscall
        j _read_value_loop
_read_value_success:
# read int is already in v0, don't need to move it
        lw      $ra, 0($sp)
        lw      $s0, 4($sp)
        lw      $s1, 8($sp)
        lw      $s2, 12($sp)
        lw      $s3, 16($sp)
        lw      $s4, 20($sp)
        lw      $s5, 24($sp)
        lw      $s6, 28($sp)
        lw      $s7, 32($sp)
        addi    $sp, $sp, FULL_FRAME
        jr      $ra


####
#
# Reads cells from stdin
# a0 contains the number of cells to read
# a1 contains the ascii value of the colony to read in (A or B)
# a2 contains a pointer to the prompt string
# a3 contains a pointer to the error string
# returns nothing
# 
####
read_cells:
        addi    $sp, $sp, -FULL_FRAME
        sw      $ra, 0($sp)
        sw      $s0, 4($sp)
        sw      $s1, 8($sp)
        sw      $s2, 12($sp)
        sw      $s3, 16($sp)
        sw      $s4, 20($sp)
        sw      $s5, 24($sp)
        sw      $s6, 28($sp)
        sw      $s7, 32($sp)

        move    $s0, $a0
        move    $s1, $a1
        move    $s2, $a2
        move    $s3, $a3

# Whoops, forgot about board size in the arguments
# So I'll pull it from globals (was trying not to but alas)
        la      $t0, config
        lw      $s4, 0($t0)

        move    $a0, $s2
        li      $v0, PRINT_STRING
        syscall

        la      $s5, scratch_board

_read_cell_loop:
        beq $s0, $zero, _read_cell_exit

# Read row
        li      $v0, READ_INT
        syscall
        move    $s7, $v0
        bne     $v1, $zero, _read_cell_failure
        slt     $t0, $s7, $s4
        beq     $t0, $zero, _read_cell_failure
        slt     $t0, $s7, $zero
        bne     $t0, $zero, _read_cell_failure

# read col
        li      $v0, READ_INT
        syscall
        move    $s6, $v0
        bne     $v1, $zero, _read_cell_failure
        slt     $t0, $s6, $s4
        beq     $t0, $zero, _read_cell_failure
        slt     $t0, $s6, $zero
        bne     $t0, $zero, _read_cell_failure

# calculate index
        mul     $t0, $s7, $s4
        add     $t0, $t0, $s6

# character address
        add     $t1, $t0, $s5
        lb      $t2, 0($t1)
        li      $t3, SPACE

# break if cell not empty
        bne     $t2, $t3, _read_cell_failure
# write colony character
        sb      $s1, 0($t1)

        addi    $s0, $s0, -1
        j       _read_cell_loop


_read_cell_failure:
        move    $a0, $s3
        li      $v0, PRINT_STRING
        syscall
        li      $a0, 1
        li      $v0, EXIT2
        syscall

_read_cell_exit:

        lw      $ra, 0($sp)
        lw      $s0, 4($sp)
        lw      $s1, 8($sp)
        lw      $s2, 12($sp)
        lw      $s3, 16($sp)
        lw      $s4, 20($sp)
        lw      $s5, 24($sp)
        lw      $s6, 28($sp)
        lw      $s7, 32($sp)
        addi    $sp, $sp, FULL_FRAME
        jr      $ra

####
#
# Copies the board in scratch_board to the main board.
# Takes no arguments
# Returns nothing
# 
####
copy_board:
        la      $t8, board
        la      $t9, scratch_board

        li      $t0, 0
_copy_board_loop:
# copy the entire buffer, makes the loop simpler
        slti    $t1, $t0, 900
        beq     $t1, $zero, _copy_board_exit
# calculate addresses
        add     $t1, $t0, $t8
        add     $t2, $t0, $t9
        lb      $t3, 0($t2)
        sb      $t3, 0($t1)
        addi    $t0, $t0, 1
        j _copy_board_loop

_copy_board_exit:
        jr      $ra


####
#
# Prints the current board, along with generation header
# a0 is a pointer to the board to print
# a1 is the size of the board
# a2 is the current generation
#
####
print_board:
        addi    $sp, $sp, -FULL_FRAME
        sw      $ra, 0($sp)
        sw      $s0, 4($sp)
        sw      $s1, 8($sp)
        sw      $s2, 12($sp)
        sw      $s3, 16($sp)
        sw      $s4, 20($sp)
        sw      $s5, 24($sp)
        sw      $s6, 28($sp)
        sw      $s7, 32($sp)

        move    $s0, $a0
        move    $s1, $a1
        move    $s2, $a2


# print header
        li      $v0, PRINT_CHAR
        li      $a0, NEWLINE
        syscall
        li      $v0, PRINT_STRING
        la      $a0, gen_header_start
        syscall
        li      $v0, PRINT_INT
        move    $a0, $s2
        syscall
        li      $v0, PRINT_STRING
        la      $a0, gen_header_end
        syscall

# print top edge
        li      $v0, PRINT_CHAR
        li      $a0, PLUS
        syscall
        li      $v0, PRINT_STRING
        la      $a0, board_edge
        syscall
        li      $v0, PRINT_CHAR
        li      $a0, PLUS
        syscall
        li      $v0, PRINT_CHAR
        li      $a0, NEWLINE
        syscall

        li      $s3, 0
_row_print_loop:
        beq     $s3, $s1, _print_exit

        li      $v0, PRINT_CHAR
        li      $a0, PIPE
        syscall

# Calc string index
        mul     $s4, $s3, $s1
        add     $s4, $s4, $s0
        add     $s5, $s4, $s1

# Cursed null terminator hacking
        lb      $s7, 0($s5)
        sb      $zero, 0($s5)

        li      $v0, PRINT_STRING
        move    $a0, $s4
        syscall

        sb      $s7, 0($s5)

        li      $v0, PRINT_CHAR
        li      $a0, PIPE
        syscall
        li      $v0, PRINT_CHAR
        li      $a0, NEWLINE
        syscall

        addi    $s3, $s3, 1
        j _row_print_loop

_print_exit:
# print bottom edge
        li      $v0, PRINT_CHAR
        li      $a0, PLUS
        syscall
        li      $v0, PRINT_STRING
        la      $a0, board_edge
        syscall
        li      $v0, PRINT_CHAR
        li      $a0, PLUS
        syscall
        li      $v0, PRINT_CHAR
        li      $a0, NEWLINE
        syscall
        
        lw      $ra, 0($sp)
        lw      $s0, 4($sp)
        lw      $s1, 8($sp)
        lw      $s2, 12($sp)
        lw      $s3, 16($sp)
        lw      $s4, 20($sp)
        lw      $s5, 24($sp)
        lw      $s6, 28($sp)
        lw      $s7, 32($sp)
        addi    $sp, $sp, FULL_FRAME
        jr      $ra

####
#
# Advances the board one generation
# Takes no arguments
# Returns nothing
#
####
next_generation:
        addi    $sp, $sp, -FULL_FRAME
        sw      $ra, 0($sp)
        sw      $s0, 4($sp)
        sw      $s1, 8($sp)
        sw      $s2, 12($sp)
        sw      $s3, 16($sp)
        sw      $s4, 20($sp)
        sw      $s5, 24($sp)
        sw      $s6, 28($sp)
        sw      $s7, 32($sp)
        
        la      $s2, config
        lw      $s2, 0($s2)
        la      $s3, board
        la      $s4, scratch_board
        

        li      $s0, 0
_gen_row_loop:
        beq     $s0, $s2, _gen_row_exit
        li      $s1, 0
_gen_col_loop:
        beq     $s1, $s2, _gen_col_exit
        
        mul     $s5, $s0, $s2
        add     $s5, $s5, $s1           # s5 is board index
        add     $s6, $s5, $s3           # s6 is board address
        add     $s7, $s5, $s4           # s7 is scratch address

        move    $a0, $s0
        move    $a1, $s1
        move    $a2, $s2 
        jal     value_at_cell
        bne     $v1, $zero, _cell_dead
# at this point abs(v0) is either 2 or 3. 
        li      $t9, A
        li      $t0, 3
        beq     $v0, $t0, _gen_inner_loop_end
        li      $t9, B
        li      $t0, -3
        beq     $v0, $t0, _gen_inner_loop_end
# at this point abs(v0) is 2, need to figure out whether cell is same align as
# neighbor majority, to determine if lives or dies
        lb      $t0, 0($s6)
        andi    $t1, $t0, 0x01          # 1 for A (65), 0 for B (66)
        slt     $t2, $zero, $v0         # 1 for A (+) , 0 for B (-) (neighbors)
        bne     $t1, $t2, _cell_dead    # cell is different from neighbors
        move    $t9, $t0
        j       _gen_inner_loop_end
_cell_dead:
        li      $t9, SPACE
        j       _gen_inner_loop_end
_gen_inner_loop_end:
# every path leading to here sets $t9 to what should be in this cell
        sb      $t9, 0($s7)
        addi    $s1, $s1, 1
        j       _gen_col_loop
_gen_col_exit:
        addi    $s0, $s0, 1
        j       _gen_row_loop
_gen_row_exit:

        jal     copy_board

        lw      $ra, 0($sp)
        lw      $s0, 4($sp)
        lw      $s1, 8($sp)
        lw      $s2, 12($sp)
        lw      $s3, 16($sp)
        lw      $s4, 20($sp)
        lw      $s5, 24($sp)
        lw      $s6, 28($sp)
        lw      $s7, 32($sp)
        addi    $sp, $sp, FULL_FRAME
        jr      $ra

####
#
# Finds the value at the current cell
# a0 is row
# a1 is column
# a2 is board size
# returns value at cell in v0
# returns 'guaranteed dead' flag in v1
####
value_at_cell:  
        li      $t9, -1         # loop counter
        li      $v0, 0          # accumulator
        la      $t0, dr
        la      $t1, dc
        la      $t2, board
_calc_value_loop:
        addi    $t9, $t9, 1     # increment at beginning to make jumps simpler
        slti    $t7, $t9, 8
        beq     $t7, $zero, _calc_value_loop_exit
# calc row
        add     $t3, $t0, $t9
        lb      $t3, 0($t3)
        add     $t3, $t3, $a0
        add     $t3, $t3, $a2
        rem     $t3, $t3, $a2
# calc column
        add     $t4, $t1, $t9
        lb      $t4, 0($t4)
        add     $t4, $t4, $a1
        add     $t4, $t4, $a2
        rem     $t4, $t4, $a2
# board index
        mul     $t3, $t3, $a2
        add     $t3, $t3, $t4
# indexed address
        add     $t3, $t3, $t2
        lb      $t4, 0($t3)

        li      $t5, A
        beq     $t4, $t5, _calc_increment
        li      $t5, B
        beq     $t4, $t5, _calc_decrement
        j _calc_value_loop
_calc_increment:
        addi    $v0, $v0, 1
        j       _calc_value_loop
_calc_decrement:
        addi    $v0, $v0, -1
        j       _calc_value_loop
_calc_value_loop_exit:
# cells will only be alive in the next generation if they have exactly 2 or 3
# neighbors, so calculate a flag to determine if we are outside that range
        move    $t7, $v0
# these next 3 instructions calculate absolute value via arcane magicks
# sra fills t6 with sign of t7, aka 0 if t7 >= 0, -1 if t7 < 0.
        sra     $t6, $t7, 31
# xor inverts the number if negative
        xor     $t7, $t7, $t6
# sub will subtract t6, which will be -1 if t7 is negative, - -1 = + 1
        sub     $t7, $t7, $t6
# thus if t7 if less than zero, the resulting operation is (~t7) + 1, or -t7
# and if t7 is positive, the operation is (t7 ^ 0) + 0, which is just t7.
# therefore t7 is abs(v0)
        slti    $v1, $t7, 2
        li      $t9, 3
        slt     $t8, $t9, $t7
        or      $v1, $v1, $t8
# v1 is flag for v0 outside 2-3 inclusive
        jr      $ra

