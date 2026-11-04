
IoPort@ inPort = component.getPort("PORTA");
IoPin@  CsPin  = component.getPin("CS");

uint   m_steps  = 64;
uint   m_bits   = 6;
double m_resist = 1000;

void setup() // Executed when Component is created
{
    print("digpot  setup() OK"); 
}

void reset() // Executed at Simulation start
{
    inPort.setPinMode( 1 ); // input
    inPort.changeCallBack( element, true );
}

void voltChanged()
{
    bool cs = CsPin.getInpState();
    if( cs ) return;                // Chip Not selected
    
    int input = inPort.getInpState() & (m_steps-1);
    int value = input*1000/(m_steps-1);
    
    component.setLinkedValue( 0, value, 0 );
}

// --------------------------------------------
// ---------------  Properties  ---------------
// --------------------------------------------

void setResistance( double r )  // Set Potentiometer resistance
{
    m_resist = r;
    component.setLinkedValue( 0, r, 1 );
}

double getResistance()
{
    return m_resist;
}

void setBits( uint b )        // Set Potentiometer steps
{
//print( "setSteps " + s );
    if     ( b > 16 ) b = 16;
    else if( b < 1  ) b = 1;
    m_bits = b;
    
    m_steps = 2;
    for( uint i=1; i<b; i++ ) m_steps *= 2;
    
    voltChanged();
}

uint getBits()
{
    return m_bits;
}
