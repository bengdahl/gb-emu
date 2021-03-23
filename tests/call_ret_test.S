SECTION "text", ROM0[0]
    LD HL, $AA55
    
    JP Main

SetZero:
    LD B, $00
    RET

SetMinusOne:
    LD B, $FF
    RET

Main:
    LD B, $FF
    CALL SetZero
    LD [HL], B

    XOR A                   ; Set Z

    LD B, $FF
    CALL Z, SetZero
    LD [HL], B

    LD B, $00
    CALL NZ, SetMinusOne
    LD [HL], B

    INC A                   ; Unset Z

    LD B, $00
    CALL Z, SetMinusOne
    LD [HL], B

    LD B, $FF
    CALL NZ, SetZero
    LD [HL], B

    SCF                     ; Set C

    LD B, $FF
    CALL C, SetZero
    LD [HL], B

    LD B, $00
    CALL NC, SetMinusOne
    LD [HL], B

    CCF                     ; Unset C

    LD B, $00
    CALL Z, SetMinusOne
    LD [HL], B

    LD B, $FF
    CALL NZ, SetZero
    LD [HL], B