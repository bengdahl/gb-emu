SECTION "text", ROM0[0]
start:
    LD HL, $AA55
    
    XOR A
    LD B, $00
    JR Z, jp1
    LD B, $FF
jp1:
    LD [HL], B

    OR A, $FF
    LD B, $FF
    JR Z, jp2
    LD B, $00
jp2:
    LD [HL], B

    XOR A
    LD B, $FF
    JR NZ, jp3
    LD B, $00
jp3:
    LD [HL], B

    OR A, $FF
    LD B, $00
    JR NZ, jp4
    LD B, $FF
jp4:
    LD [HL], B

    SCF
    LD B, $00
    JR C, jp5
    LD B, $FF
jp5:
    LD [HL], B

    SCF
    CCF
    LD B, $FF
    JR C, jp6
    LD B, $00
jp6:
    LD [HL], B

    SCF
    LD B, $FF
    JR NC, jp7
    LD B, $00
jp7:
    LD [HL], B

    SCF
    CCF
    LD B, $00
    JR NC, jp8
    LD B, $FF
jp8:
    LD [HL], B

    JP test_page_bound

SECTION "page_bound_test", ROM0[$AF]
test_page_bound:
    JR jp_page_two

final: 
    LD B, $20
    LD [HL], B
spin:
    JR spin

SECTION "page_two", ROM0[$FF]
jp_page_two:
    LD B, $10
    LD [HL], B
    JR final