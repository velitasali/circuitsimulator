
// PinModes: undef_mode=0, input, openCo, output, source

IoPort@ outputPort = component.getPort("PORTA");

enum links{
    Dial=0,
    Label
}

enum code_t{
    HEX=0,
    BCD,
    GRAY
}

array<uint> m_grayCode(15);

uint m_value;
uint m_code;
string m_codeStr;
bool m_changed;

void setup()
{
    //print("ENCODER_script init");
    m_grayCode = {0,1,3,2,6,7,5,4,12,13,15,14,10,11,9,8};
    m_value = 0;
    m_code = HEX;
    m_codeStr = "HEX";
}

void reset()
{
    //print("resetting ENCODER_script"); 
    
    outputPort.setPinMode( 3 );  // Output
    outputPort.setOutState( m_value ); 
}

void updateStep()
{
    if( !m_changed ) return;
    m_changed = false;
    
    if( m_code == GRAY ) outputPort.setOutState( m_grayCode[m_value] );
    else                 outputPort.setOutState( m_value );
}

void setLinkedValue( double val, int i )
{
    // val = 0 to 
    m_value = uint(val);
    m_changed = true;
    //print("ENCODER_script setLinkedValue "+val);
}

void setCode( string c )
{
    //print( "setCode "+c );
    
    if( m_codeStr == c ) return;
    m_codeStr = c;
    
    uint steps = 16;
    
    if( c == "BCD" )
    {
        steps = 10;
        m_code = BCD;
        component.setLinkedString( 1, "09", 0 );
    }
    else if( c == "GRAY" )
    {
        m_code = GRAY;
        component.setLinkedString( 1, "15", 0 );
    }
    else     // HEX
    {
        m_code = HEX;
        component.setLinkedString( 1, "15", 0 );
    }
    
    string stepsStr = formatUInt( steps );
    component.setPropStr( 0, "Steps", stepsStr );
    
    string maxValStr = formatUInt( steps-1 );
    component.setPropStr( 0, "Max_Val", maxValStr );
}

string getCode()
{
    return m_codeStr;
}

void setValue( uint v )
{
    m_value = v;
    
    string valueStr = formatUInt( m_value );
    component.setPropStr( Dial, "Value", valueStr );
    
    m_changed = true;
    //print("ENCODER_script setValue "+v);
}

uint getValue()
{
    //print("ENCODER_script getValue "+m_value);
    return m_value;
}
