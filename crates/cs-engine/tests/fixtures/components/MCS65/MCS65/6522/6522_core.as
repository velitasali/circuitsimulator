
McuPort@ PortA   = component.getMcuPort("PORTA");
McuPort@ PortB   = component.getMcuPort("PORTB");

IoPort@ csPort   = component.getPort("PORTC");
IoPort@ ctrlPort = component.getPort("PORTE");
IoPort@ dataPort = component.getPort("PORTD");
IoPort@ addrPort = component.getPort("PORTR");

IoPin@ ca1Pin  = component.getPin("CA1");
IoPin@ ca2Pin  = component.getPin("CA2");
IoPin@ cb1Pin  = component.getPin("CB1");
IoPin@ cb2Pin  = component.getPin("CB2");
IoPin@ irqPin  = component.getPin("IRQ");
IoPin@ rwPin   = component.getPin("RW");

enum pinModes{
    undef_mode=0, 
    input,
    openCo,
    output,
    source
}

const uint m_rDelay = 10000; // 10 ns

uint ORB;
uint ORA;
uint DDRB;
uint DDRA;
uint T1CL;   //Timer 1 counter low
uint T1CH;   //Timer 1 counter high
uint T1LL;   // Timer 1 Latch Low
uint T1LH;   // Timer 1 Latch High
uint T2CL;
uint T2CH;
uint SR;
uint ACR;    // AuxiliaryControl Register
uint PCR;    // Peripheral Control Register
uint IFR;
uint IER;


uint IRB;
uint IRA;

enum acrBits{
    PA   = 1<<0,
    PB   = 1<<1,
    SRC0 = 1<<2,
    SRC1 = 1<<3,
    SRC2 = 1<<4,
    T2C  = 1<<5,
    T1C0 = 1<<6,
    T1C1 = 1<<7
}
enum pcrBits{
    CA1c = 1<<0,
    CA20 = 1<<1,
    CA21 = 1<<2,
    CA22 = 1<<3,
    CB1c = 1<<4,
    CB20 = 1<<5,
    CB21 = 1<<6,
    CB22 = 1<<7
}
enum ifrBits{
    CA2IF = 1<<0,
    CA1IF = 1<<1,
    SRIF  = 1<<2,
    CB2IF = 1<<3,
    CB1IF = 1<<4,
    T2IF  = 1<<5,
    T1IF  = 1<<6,
    IRQIF = 1<<7
}
enum ctrlBits{
    CA1bit = 1<<0,
    CA2bit = 1<<1,
    CB1bit = 1<<2,
    CB2bit = 1<<3
}


uint m_addr;

bool m_nextClock;  // Clock State
bool m_read;
bool m_CS;

bool m_latchA;
bool m_latchB;
bool m_pulseCA2;
bool m_pulseCB2;

int m_CA2ctrl;
int m_CB2ctrl;
uint m_ctrlState;
uint m_ctrlFlags;

bool m_T2ctrl;
bool m_T1cont;
bool m_T1pb7;

void setup() // Executed at setScript()
{
    print("6522 setup() OK"); 
}

void reset() // Executed at Simulation start
{ 
    print("6522 reset()");
    
    irqPin.setPinMode( openCo );
    irqPin.setOutState( true );
    dataPort.setPinMode( input );
    
    ca2Pin.setOutState( true );
    csPort.changeCallBack( element, true );
    ctrlPort.changeCallBack( element, true );
    rwPin.changeCallBack( element, true );

    m_nextClock = true; // Wait for first rising edge
    
    m_read = false;
    m_CS = false;
    
    m_pulseCA2 = false;
    m_pulseCB2 = false;
    
    m_latchA = false;
    m_latchB = false;
    
    m_CA2ctrl = -1;
    m_CB2ctrl = -1;
    m_ctrlState = 0;
    m_ctrlFlags = 0;
    
    ORB  = 0;
    ORA  = 0;
    DDRB = 0;
    DDRA = 0;
    T1CL = 0;
    T1CH = 0;
    T1LL = 0;
    T1LH = 0;
    T2CL = 0;
    T2CH = 0;
    SR   = 0;
    ACR  = 0;
    PCR  = 0;
    IFR  = 0;
    IER  = 1<<7;
}

void voltChanged()
{
    m_CS = csPort.getInpState() == 1;
    m_read = rwPin.getInpState();
    
    bool clkHigh = !m_nextClock;
    bool dataOut = m_CS && m_read && clkHigh;
    
    if( dataOut ) dataPort.setPinMode( output );
    else          dataPort.setPinMode( input );
    
    uint ctrlState = ctrlPort.getInpState();
    if( m_ctrlState != ctrlState )
    {
        uint changed = m_ctrlState ^ ctrlState;
        m_ctrlState = ctrlState;
        if( changed == 0 ) return;
        
        for( int i=0; i<4; i++ )
        {
            uint chgBit = changed & 1<<i;
            if( chgBit != 0 ) extInt( chgBit );
        }
    }
}

void extClock( bool clkState )  // Function called al clockPin change
{
    if( m_nextClock != clkState ) return;
    
    if( m_CS ) // Chip Selected
    {
        if( m_nextClock ) risingEdge();
        else              fallingEdge();
    }
    m_nextClock = !m_nextClock;
}

void risingEdge()
{
    m_addr = addrPort.getInpState();
    m_read = rwPin.getInpState();
    
    if( m_read )     // MCU is reading
    {
        dataPort.setPinMode( output );
        component.addEvent( m_rDelay ); // Set Data Port after a few ns
    }else{
        dataPort.setPinMode( input );   // We will read data at Falling edge
    }
}

void fallingEdge()
{
    if( !m_read ) // MCU is writing
    {
        uint data = dataPort.getInpState(); // Read Data Port
        writeREG( data );
    }
}

void runEvent()
{
    if( m_read ) // MCU is reading
    {
        uint data = readREG();
        dataPort.setOutState( data );       // Set Data Port
    }
}

uint readREG()
{
    uint data = 0;
    switch( m_addr ){
        case 0: {
            data = ORB & DDRB;                  // Out Pin: bit from Register
            if( m_latchB ) data |= IRB & ~DDRB; // latched at CB1 change
            else           data |= PortB.getInpState() & ~DDRB;// In  Pin: bit from Pin
            IRA = data;
            writeIFR( 0b00011000 ); // clear CB1/2 flags by writing 1
        }break;
        case 1:{
            if( m_latchA ) data = IRA;          // Latched at CA1 change
            else           data = PortA.getInpState();
            IRA = data;
            writeIFR( 0b00000011 ); // clear CA1/2 flags by writing 1
        }break;
        case 2: data = DDRB; break;
        case 3: data = DDRA; break;
        case 4: data = T1CL; writeIFR( 0b01000000 ); break; // T1 interrupt flag IFR6 is reset.
        case 5: data = T1CH; break;
        case 6: data = T1LL; break;
        case 7: data = T1LH; break;
        case 8: break; 
        case 9: break;
        case 10: data = SR; writeIFR( 0b00000100 ); break; // SR interrupt flag clear
        case 11: data = ACR; break;
        case 12: data = PCR; break;
        case 13: data = IFR; break;
        case 14: data = IER; break;
        default: data = component.readRAM( m_addr );
    }
    return data;
}

void writeREG( uint data )
{
    switch( m_addr ){
        case 0: ORB  = data; writeIFR( 0b00011000 ); break; // clear CB1/2 flags by writing 1
        case 1: ORA  = data; writeIFR( 0b00000011 ); break; // clear CA1/2 flags by writing 1
        case 2: DDRB = data; break;
        case 3: DDRA = data; break;
        case 4: T1LL = data; break; // T1CL Load data into latch Low
        case 5:{                    // T1CH
            T1LH = data;            // load data into latch High
            T1CH = T1LH;
            T1CL = T1LL;
            // initiates countdown
            // T1 interrupt flag IFR6 is reset. ?????????
        } break;
        case 6: T1LL = data; break; 
        case 7: T1LH = data; writeIFR( 0b01000000 ); break; // T1CH load data into latch High // T1 interrupt flag IFR6 is reset.
        case 8: break; 
        case 9: break; 
        case 10: SR = data; writeIFR( 0b00000100 ); break; // SR interrupt flag clear
        case 11: writeACR( data ); break; // ACR
        case 12: writePCR( data ); break; // PCR
        case 13: writeIFR( data ); return; // IFR
        case 14: writeIER( data ); return; // IER
    }
    component.writeRAM( m_addr, data ); // Mcu Mon, Dir changed, Int en/dis, etc.
}

void writeACR( uint v )
{
    m_latchA = (ACR & PA) > 0;
    m_latchB = (ACR & PB) > 0;
    
    m_T2ctrl = (ACR & T2C) > 0;
    
    // SR
    
    m_T1cont = (ACR & T1C0) > 0;
    m_T1pb7  = (ACR & T1C1) > 0;
}

void writePCR( uint v )
{
    PCR = v;
    
    if( (PCR & CA1c) > 0 ) m_ctrlFlags |=  CA1bit; // Interrupt at CA1 Rising edge
    else                   m_ctrlFlags &= ~CA1bit; // Interrupt at CA1 Falling edge
    
    if( (PCR & CA22) > 0 ) // CA2 Control
    {
        m_CA2ctrl = -1;
        if( (PCR & CA21) > 0 ) ca2Pin.setOutState( (PCR & CA20) > 0 ); // Manual Output
        else                   m_CA2ctrl = PCR & CA20;// Handshake/Pulse Output
    }else{
        if( (PCR & CA21) > 0 ) m_ctrlFlags |=  CA2bit; // Interrupt at CA2 Rising edge
        else                   m_ctrlFlags &= ~CA2bit; // Interrupt at CA2 Falling edge
        /// if( (PCR & CA20) > 0 ) ;//Independent interrupt
    }
    
    if( (PCR & CB1c) > 0 ) m_ctrlFlags |=  CB1bit; // Interrupt at CA1 Rising edge
    else                   m_ctrlFlags &= ~CB1bit; // Interrupt at CA1 Falling edge
    
    if( (PCR & CA22) > 0 ) // CB2 Control
    {
        m_CB2ctrl = -1;
        if( (PCR & CB21) > 0 ) cb2Pin.setOutState( (PCR & CB20) > 0 ); // Manual Output
        else                   m_CB2ctrl = PCR & CB20;// Handshake/Pulse Output
    }else{
        if( (PCR & CB21) > 0 ) m_ctrlFlags |=  CB2bit; // Interrupt at CA1 Rising edge
        else                   m_ctrlFlags &= ~CB2bit; // Interrupt at CA1 Falling edge
        /// if( (PCR & CB20) > 0 ) ;//Independent interrupt
    }
}

void writeIER( uint v )
{
    int set = v & 1<<7;
    v &= 0b01111111;
    if( set == 0 ) IER &= ~v ; // Clear flags
    else           IER |=  v ; // Set flags
    
    component.writeRAM( 14, IER );
}

void writeIFR( uint v )
{
    v &= 0b01111111;
    IFR &= ~v;
    if( IFR == 0 ) irqPin.setOutState( true );
    else           IFR |= 0b10000000;
    
    component.writeRAM( 13, IFR );
}

void extInt( uint chgBit )
{
    if( (m_ctrlState & chgBit) == m_ctrlFlags ) // Interrput
    {
        switch( chgBit )
        {
            case CA1bit: IFR |= CA1IF; break;
            case CA2bit: IFR |= CA2IF; break;
            case CB1bit: IFR |= CB1IF; break;
            case CB2bit: IFR |= CB2IF; break;
        }
        irqPin.setOutState( false );
    }
}

