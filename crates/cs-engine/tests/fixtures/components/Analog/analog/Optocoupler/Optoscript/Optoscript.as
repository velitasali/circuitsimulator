
double m_ledThreshold  = 1.8;
double m_ledMaxcurrent = 0.008;
double m_ledResistance = 0.6;

double m_fetRDSon     = 0.05;
double m_fetThreshold = 3;

enum myLinks{
    LED=0,
    MOSFET
}

void setup()
{
}

void reset()
{
}

void setFordward_Voltage( double v )
{
    if( m_ledThreshold == v ) return;
    m_ledThreshold = v;
    
    string propStr = ""+v;
    component.setPropStr( LED, "Threshold", propStr );
}

double getFordward_Voltage()
{
    return m_ledThreshold;
}

void setMax_Current( double c )
{
    if( m_ledMaxcurrent == c ) return;
    m_ledMaxcurrent = c;
    
    string propStr = ""+c+" A";
    component.setPropStr( LED, "MaxCurrent", propStr );
    print( "setMax_Current "+c);
}

double getMax_Current()
{
    return m_ledMaxcurrent;
}

void setResistance( double r )
{
    if( m_ledResistance == r ) return;
    m_ledResistance = r;
    
    string propStr = ""+r;
    component.setPropStr( LED, "Resistance", propStr );
}

double getResistance()
{
    return m_ledResistance;
}

void setRDSon( double r )
{
    if( m_fetRDSon == r ) return;
    m_fetRDSon = r;
    
    string propStr = ""+r;
    component.setPropStr( MOSFET, "Resistance", propStr );
}

double getRDSon()
{
    return m_fetRDSon;
}

void setThreshold( double v )
{
    if( m_fetThreshold == v ) return;
    m_fetThreshold = v;
    
    string propStr = ""+v;
    component.setPropStr( MOSFET, "Threshold", propStr );
}

double getThreshold()
{
    return m_fetThreshold;
}

