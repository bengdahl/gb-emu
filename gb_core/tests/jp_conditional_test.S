SECTION "text", ROM0[0]
    LD HL, $AA55
    
    XOR A
    LD B, $00
    JP Z, jp1
    LD B, $FF
jp1:
    LD [HL], B

    OR A, $FF
    LD B, $FF
    JP Z, jp2
    LD B, $00
jp2:
    LD [HL], B

    XOR A
    LD B, $FF
    JP NZ, jp3
    LD B, $00
jp3:
    LD [HL], B

    OR A, $FF
    LD B, $00
    JP NZ, jp4
    LD B, $FF
jp4:
    LD [HL], B

    SCF
    LD B, $00
    JP C, jp5
    LD B, $FF
jp5:
    LD [HL], B

    SCF
    CCF
    LD B, $FF
    JP C, jp6
    LD B, $00
jp6:
    LD [HL], B

    SCF
    LD B, $FF
    JP NC, jp7
    LD B, $00
jp7:
    LD [HL], B

    SCF
    CCF
    LD B, $00
    JP NC, jp8
    LD B, $FF
jp8:
    LD [HL], B