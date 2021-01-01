//! Line-oriented `.sim1` / `.sim2` load/save matching `Circuit::loadStrDoc`.

use crate::circ1::{GraphicAttrs, ParsedCirc1, ParsedCirc1Item};
use crate::elements::pins::*;
use crate::elements::{Comp, Kind};
use crate::settings::CircSettings;
use crate::units::{format_si, parse_si};
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct ParsedItem {
    pub comp: Comp,
    /// Scene position from `Pos="x,y"`, if present.
    pub pos: Option<(f64, f64)>,
    pub rotation: f64,
    pub hflip: i32,
    pub vflip: i32,
    pub label: Option<String>,
    pub show_id: bool,
    pub label_pos: Option<(f64, f64)>,
    pub label_rot: Option<i32>,
    pub show_val: bool,
    pub show_prop: Option<String>,
    pub val_pos: Option<(f64, f64)>,
    pub val_rot: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct ParsedConnector {
    pub id: String,
    pub start: String,
    pub end: String,
    pub points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct ParsedCircuit {
    pub items: Vec<ParsedItem>,
    pub connectors: Vec<ParsedConnector>,
    pub skipped: Vec<String>,
    pub analog_dt: f64,
    pub max_nl_steps: u32,
    pub circ: CircSettings,
}

pub const TAG_CIRCUIT: &str = "<circuit";
pub const TAG_ITEM: &str = "<item";

/// Legacy XML item type constants.
pub const ITEM_ADC: &str = "Adc";
pub const ITEM_ADC_UPPER: &str = "ADC";
pub const ITEM_AIP31068_LOWER: &str = "aip31068";
pub const ITEM_AIP31068_UPPER: &str = "AIP31068";
pub const ITEM_AMMETER: &str = "Ammeter";
pub const ITEM_AMPERIMETER: &str = "Amperimeter";
pub const ITEM_ANALOGMUX: &str = "AnalogMux";
pub const ITEM_ANALOG_MUX: &str = "Analog Mux";
pub const ITEM_ANALOG_MUX_LOWER: &str = "analog_mux";
pub const ITEM_ANDGATE: &str = "AndGate";
pub const ITEM_AND_GATE: &str = "And Gate";
pub const ITEM_AUDIOOUT: &str = "AudioOut";
pub const ITEM_BATTERY: &str = "Battery";
pub const ITEM_BCDTO7S: &str = "BcdTo7S";
pub const ITEM_BCDTODEC: &str = "BcdToDec";
pub const ITEM_BINCOUNTER: &str = "BinCounter";
pub const ITEM_BJT: &str = "Bjt";
pub const ITEM_BJT_LOWER: &str = "bjt";
pub const ITEM_BJT_UPPER: &str = "BJT";
pub const ITEM_BUFFER: &str = "Buffer";
pub const ITEM_BUFFERGATE: &str = "BufferGate";
pub const ITEM_BUS: &str = "Bus";
pub const ITEM_CAPACITOR: &str = "Capacitor";
pub const ITEM_CLOCK: &str = "Clock";
pub const ITEM_COMPARATOR: &str = "Comparator";
pub const ITEM_COMPARATOR_LOWER: &str = "comparator";
pub const ITEM_CONNECTOR: &str = "Connector";
pub const ITEM_COUNTER: &str = "Counter";
pub const ITEM_CSOURCE: &str = "Csource";
pub const ITEM_CURRENT_SOURCE: &str = "Current Source";
pub const ITEM_CURRSOURCE: &str = "CurrSource";
pub const ITEM_DAC: &str = "Dac";
pub const ITEM_DAC_UPPER: &str = "DAC";
pub const ITEM_DCMOTOR: &str = "DcMotor";
pub const ITEM_DCMOTOR_LOWER: &str = "dcmotor";
pub const ITEM_DECTOBCD: &str = "DecToBcd";
pub const ITEM_DEMUX: &str = "Demux";
pub const ITEM_DHT11: &str = "Dht11";
pub const ITEM_DHT11_LOWER: &str = "dht11";
pub const ITEM_DHT11_UPPER: &str = "DHT11";
pub const ITEM_DHT22: &str = "Dht22";
pub const ITEM_DHT22_LOWER: &str = "dht22";
pub const ITEM_DHT22_UPPER: &str = "DHT22";
pub const ITEM_DIAC: &str = "Diac";
pub const ITEM_DIAC_LOWER: &str = "diac";
pub const ITEM_DIAC_UPPER: &str = "DIAC";
pub const ITEM_DIAL: &str = "Dial";
pub const ITEM_DIODE: &str = "Diode";
pub const ITEM_DIODE_LOWER: &str = "diode";
pub const ITEM_DS1307_LOWER: &str = "ds1307";
pub const ITEM_DS1307_UPPER: &str = "DS1307";
pub const ITEM_DS1621_LOWER: &str = "ds1621";
pub const ITEM_DS1621_UPPER: &str = "DS1621";
pub const ITEM_DS18B20: &str = "Ds18b20";
pub const ITEM_DS18B20_LOWER: &str = "ds18b20";
pub const ITEM_DS18B20_UPPER: &str = "DS18B20";
pub const ITEM_DYNAMICMEMORY: &str = "DynamicMemory";
pub const ITEM_ELCAPACITOR: &str = "ElCapacitor";
pub const ITEM_ELCAPACITOR_1: &str = "elCapacitor";
pub const ITEM_ELLIPSE: &str = "Ellipse";
pub const ITEM_ESP01: &str = "Esp01";
pub const ITEM_ESP01_LOWER: &str = "esp01";
pub const ITEM_FIXEDVOLT: &str = "FixedVolt";
pub const ITEM_FIXED_VOLTAGE: &str = "Fixed Voltage";
pub const ITEM_FLIPFLOPD: &str = "FlipFlopD";
pub const ITEM_FLIPFLOPJK: &str = "FlipFlopJK";
pub const ITEM_FLIPFLOP_RS: &str = "FlipFlop RS";
pub const ITEM_FLIPFLOP_T: &str = "FlipFlop T";
pub const ITEM_FREQMETER: &str = "FreqMeter";
pub const ITEM_FREQUENCIMETER: &str = "Frequencimeter";
pub const ITEM_FULLADDER: &str = "FullAdder";
pub const ITEM_FUNCTION: &str = "Function";
pub const ITEM_GC9A01A_UPPER: &str = "GC9A01A";
pub const ITEM_GROUND: &str = "Ground";
pub const ITEM_HC_SR04_UPPER: &str = "HC-SR04";
pub const ITEM_HD44780: &str = "Hd44780";
pub const ITEM_HEADER: &str = "Header";
pub const ITEM_I2CRAM: &str = "I2CRam";
pub const ITEM_I2CTOPARALLEL: &str = "I2CToParallel";
pub const ITEM_ILI9341_UPPER: &str = "ILI9341";
pub const ITEM_IMAGE: &str = "Image";
pub const ITEM_INDUCTOR: &str = "Inductor";
pub const ITEM_JFET: &str = "Jfet";
pub const ITEM_JFET_LOWER: &str = "jfet";
pub const ITEM_JFET_UPPER: &str = "JFET";
pub const ITEM_KEYPAD: &str = "KeyPad";
pub const ITEM_KS0108_LOWER: &str = "ks0108";
pub const ITEM_KS0108_UPPER: &str = "KS0108";
pub const ITEM_KY023_LOWER: &str = "ky023";
pub const ITEM_KY023_UPPER: &str = "KY023";
pub const ITEM_KY040_LOWER: &str = "ky040";
pub const ITEM_KY040_UPPER: &str = "KY040";
pub const ITEM_KY_023_UPPER: &str = "KY-023";
pub const ITEM_KY_040_UPPER: &str = "KY-040";
pub const ITEM_LAMP: &str = "Lamp";
pub const ITEM_LANALIZER: &str = "LAnalizer";
pub const ITEM_LATCHD: &str = "LatchD";
pub const ITEM_LDR_LOWER: &str = "ldr";
pub const ITEM_LDR_UPPER: &str = "LDR";
pub const ITEM_LED: &str = "Led";
pub const ITEM_LEDBAR: &str = "LedBar";
pub const ITEM_LEDMATRIX: &str = "LedMatrix";
pub const ITEM_LED_LOWER: &str = "led";
pub const ITEM_LED_UPPER: &str = "LED";
pub const ITEM_LINE: &str = "Line";
pub const ITEM_LM555: &str = "Lm555";
pub const ITEM_LM555_UPPER: &str = "LM555";
pub const ITEM_LOGICANALYZER: &str = "LogicAnalyzer";
pub const ITEM_MAGNITUDECOMP: &str = "MagnitudeComp";
pub const ITEM_MAX72XX: &str = "Max72xx";
pub const ITEM_MCU: &str = "Mcu";
pub const ITEM_MCU_UPPER: &str = "MCU";
pub const ITEM_MEMORY: &str = "Memory";
pub const ITEM_MOSFET: &str = "Mosfet";
pub const ITEM_MOSFET_LOWER: &str = "mosfet";
pub const ITEM_MOSFET_UPPER: &str = "MOSFET";
pub const ITEM_MUX: &str = "Mux";
pub const ITEM_MUXANALOG: &str = "MuxAnalog";
pub const ITEM_MUX_ANALOG_LOWER: &str = "mux_analog";
pub const ITEM_NODE: &str = "Node";
pub const ITEM_NOKIA5110: &str = "Nokia5110";
pub const ITEM_NUM_555: &str = "555";
pub const ITEM_OPAMP: &str = "opAmp";
pub const ITEM_OPAMP_1: &str = "OpAmp";
pub const ITEM_OPAMP_LOWER: &str = "opamp";
pub const ITEM_ORGATE: &str = "OrGate";
pub const ITEM_OR_GATE: &str = "Or Gate";
pub const ITEM_OSCILLOSCOPE: &str = "Oscilloscope";
pub const ITEM_OSCOPE: &str = "Oscope";
pub const ITEM_PACKAGE: &str = "Package";
pub const ITEM_PCD8544_LOWER: &str = "pcd8544";
pub const ITEM_PCD8544_UPPER: &str = "PCD8544";
pub const ITEM_PCF8833_LOWER: &str = "pcf8833";
pub const ITEM_PCF8833_UPPER: &str = "PCF8833";
pub const ITEM_POTENTIOMETER: &str = "Potentiometer";
pub const ITEM_POTENTIOMETER_LOWER: &str = "potentiometer";
pub const ITEM_PROBE: &str = "Probe";
pub const ITEM_PUSH: &str = "Push";
pub const ITEM_QEMUDEVICE: &str = "QemuDevice";
pub const ITEM_RAIL: &str = "Rail";
pub const ITEM_RECTANGLE: &str = "Rectangle";
pub const ITEM_RELAY: &str = "Relay";
pub const ITEM_RESISTOR: &str = "Resistor";
pub const ITEM_RESISTORDIP: &str = "ResistorDip";
pub const ITEM_RESISTOR_DIP_LOWER: &str = "resistor_dip";
pub const ITEM_RGBLED: &str = "RGBLed";
pub const ITEM_RTD_LOWER: &str = "rtd";
pub const ITEM_RTD_UPPER: &str = "RTD";
pub const ITEM_SCR_LOWER: &str = "scr";
pub const ITEM_SCR_UPPER: &str = "SCR";
pub const ITEM_SDCARD: &str = "SdCard";
pub const ITEM_SDCARD_LOWER: &str = "sdcard";
pub const ITEM_SERIALPORT: &str = "SerialPort";
pub const ITEM_SERIALTERM: &str = "SerialTerm";
pub const ITEM_SERVO: &str = "Servo";
pub const ITEM_SERVO_LOWER: &str = "servo";
pub const ITEM_SEVENSEGMENT: &str = "SevenSegment";
pub const ITEM_SEVENSEGMENTBCD: &str = "SevenSegmentBCD";
pub const ITEM_SH1107_LOWER: &str = "sh1107";
pub const ITEM_SH1107_UPPER: &str = "SH1107";
pub const ITEM_SHIFTREG: &str = "ShiftReg";
pub const ITEM_SOCKET: &str = "Socket";
pub const ITEM_SR04_LOWER: &str = "sr04";
pub const ITEM_SR04_UPPER: &str = "SR04";
pub const ITEM_SSD1306: &str = "Ssd1306";
pub const ITEM_ST7735_UPPER: &str = "ST7735";
pub const ITEM_ST7789_UPPER: &str = "ST7789";
pub const ITEM_STEPPER: &str = "Stepper";
pub const ITEM_STEPPER_LOWER: &str = "stepper";
pub const ITEM_STRAIN: &str = "Strain";
pub const ITEM_STRAIN_LOWER: &str = "strain";
pub const ITEM_SUBCIRCUIT: &str = "Subcircuit";
pub const ITEM_SUBPACKAGE: &str = "SubPackage";
pub const ITEM_SWITCH: &str = "Switch";
pub const ITEM_SWITCHDIP: &str = "SwitchDip";
pub const ITEM_TESTUNIT: &str = "TestUnit";
pub const ITEM_TEXT: &str = "Text";
pub const ITEM_TEXTCOMPONENT: &str = "TextComponent";
pub const ITEM_TFTDISPLAY: &str = "TFTDisplay";
pub const ITEM_TFT_DISPLAY_LOWER: &str = "tft_display";
pub const ITEM_THERMISTOR: &str = "Thermistor";
pub const ITEM_THERMISTOR_LOWER: &str = "thermistor";
pub const ITEM_TOUCHPAD: &str = "TouchPad";
pub const ITEM_TOUCHPADR: &str = "TouchPadR";
pub const ITEM_TOUCHPAD_LOWER: &str = "touchpad";
pub const ITEM_TRANSFORMER: &str = "Transformer";
pub const ITEM_TRANSFORMER_LOWER: &str = "transformer";
pub const ITEM_TRIAC: &str = "Triac";
pub const ITEM_TRIAC_LOWER: &str = "triac";
pub const ITEM_TRIAC_UPPER: &str = "TRIAC";
pub const ITEM_TUNNEL: &str = "Tunnel";
pub const ITEM_VARCAPACITOR: &str = "VarCapacitor";
pub const ITEM_VARINDUCTOR: &str = "VarInductor";
pub const ITEM_VARRESISTOR: &str = "VarResistor";
pub const ITEM_VAR_CAPACITOR_LOWER: &str = "var_capacitor";
pub const ITEM_VAR_INDUCTOR_LOWER: &str = "var_inductor";
pub const ITEM_VAR_RESISTOR_LOWER: &str = "var_resistor";
pub const ITEM_VOLTAGE_SOURCE: &str = "Voltage Source";
pub const ITEM_VOLTIMETER: &str = "Voltimeter";
pub const ITEM_VOLTMETER: &str = "Voltmeter";
pub const ITEM_VOLTREG: &str = "VoltReg";
pub const ITEM_VOLTREGULATOR: &str = "VoltRegulator";
pub const ITEM_VOLTREG_LOWER: &str = "voltreg";
pub const ITEM_VOLTSOURCE: &str = "VoltSource";
pub const ITEM_VOLT_REGULATOR: &str = "Volt. Regulator";
pub const ITEM_WAVEGEN: &str = "WaveGen";
pub const ITEM_WS2812: &str = "Ws2812";
pub const ITEM_WS2812_UPPER: &str = "WS2812";
pub const ITEM_XORGATE: &str = "XorGate";
pub const ITEM_XOR_GATE: &str = "Xor Gate";
pub const ITEM_ZENER: &str = "Zener";
pub const ITEM_ZENER_DIODE: &str = "Zener Diode";
pub const ITEM_ZENER_DIODE_1: &str = "Zener diode";
pub const ITEM_ZENER_LOWER: &str = "zener";

/// Legacy XML property name constants.
pub const PROP_ACTIVE_LOW: &str = "Active_Low";
pub const PROP_ADDRESS: &str = "Address";
pub const PROP_ADDR_BITS: &str = "Addr_Bits";
pub const PROP_AMPLITUDE: &str = "Amplitude";
pub const PROP_ANICURR_LOWER: &str = "anicurr";
pub const PROP_ANIMATE_LOWER: &str = "animate";
pub const PROP_ANSI_LOWER: &str = "ansi";
pub const PROP_ARGS: &str = "Args";
pub const PROP_AUTO_LOAD: &str = "Auto_Load";
pub const PROP_BAUD: &str = "Baud";
pub const PROP_BAUDRATE: &str = "Baudrate";
pub const PROP_BCKGND_DATA: &str = "BckGndData";
pub const PROP_BGR: &str = "BGR";
pub const PROP_BIAS: &str = "Bias";
pub const PROP_BIPOLAR: &str = "Bipolar";
pub const PROP_BITS: &str = "Bits";
pub const PROP_BORDER: &str = "Border";
pub const PROP_BREAKOVER: &str = "Breakover";
pub const PROP_BRKDOWNV: &str = "BrkDownV";
pub const PROP_BRKVOLT: &str = "BrkVolt";
pub const PROP_BUSSED: &str = "Bussed";
pub const PROP_BUZZER: &str = "Buzzer";
pub const PROP_CAPACITANCE: &str = "Capacitance";
pub const PROP_CHANNELS: &str = "Channels";
pub const PROP_CHECKED: &str = "Checked";
pub const PROP_CIRCID: &str = "CircId";
pub const PROP_COLOR: &str = "Color";
pub const PROP_COLS: &str = "Cols";
pub const PROP_COMMON_ANODE: &str = "Common_Anode";
pub const PROP_COMMON_PIN: &str = "Common_Pin";
pub const PROP_CONNECTGND: &str = "connectGnd";
pub const PROP_CONTRAST: &str = "Contrast";
pub const PROP_CONTROLLER: &str = "Controller";
pub const PROP_CONTROL_CODE: &str = "Control_Code";
pub const PROP_COUNT: &str = "Count";
pub const PROP_COUPLING: &str = "Coupling";
pub const PROP_DATA_BITS: &str = "Data_Bits";
pub const PROP_DEBUG: &str = "Debug";
pub const PROP_DEPLETION: &str = "Depletion";
pub const PROP_DEVICE: &str = "Device";
pub const PROP_DEVICE_LOWER: &str = "device";
pub const PROP_DEV_ADDRESS: &str = "Dev_Address";
pub const PROP_DIODES: &str = "Diodes";
pub const PROP_DIR: &str = "Dir";
pub const PROP_DISTANCE: &str = "Distance";
pub const PROP_DOUBLE_THROW: &str = "Double_Throw";
pub const PROP_DUTY: &str = "Duty";
pub const PROP_EMBEED_BCK: &str = "Embeed_bck";
pub const PROP_EMCOEF: &str = "EmCoef";
pub const PROP_ENDPINID_LOWER: &str = "endpinid";
pub const PROP_EXPRESSION: &str = "Expression";
pub const PROP_FILE: &str = "File";
pub const PROP_FILTER: &str = "Filter";
pub const PROP_FIXED_WIDTH: &str = "Fixed_Width";
pub const PROP_FLOATING: &str = "Floating";
pub const PROP_FONT: &str = "Font";
pub const PROP_FONT_COLOR: &str = "Font_Color";
pub const PROP_FONT_SIZE: &str = "Font_Size";
pub const PROP_FORCEFREQ: &str = "ForceFreq";
pub const PROP_FREQ: &str = "Freq";
pub const PROP_FREQUENCY: &str = "Frequency";
pub const PROP_GAIN: &str = "Gain";
pub const PROP_GATE_TH: &str = "Gate_Th";
pub const PROP_GROUNDED: &str = "Grounded";
pub const PROP_HEIGHT: &str = "Height";
pub const PROP_HEIGHT_LOWER: &str = "height";
pub const PROP_HFLIP_LOWER: &str = "hflip";
pub const PROP_HOLDCURR: &str = "HoldCurr";
pub const PROP_H_SIZE: &str = "H_size";
pub const PROP_HUMI: &str = "Humi";
pub const PROP_IDLABPOS: &str = "idLabPos";
pub const PROP_IDSS: &str = "Idss";
pub const PROP_IMAGE_FILE: &str = "Image_File";
pub const PROP_IMPEDANCE: &str = "Impedance";
pub const PROP_INDUCTANCE: &str = "Inductance";
pub const PROP_INDUCTANCE1: &str = "Inductance1";
pub const PROP_INDUCTANCE2: &str = "Inductance2";
pub const PROP_INITHIGH: &str = "initHigh";
pub const PROP_INPUTIMPED: &str = "InputImped";
pub const PROP_INPUTS: &str = "Inputs";
pub const PROP_INPUT_HIGH_V: &str = "Input_High_V";
pub const PROP_INPUT_LOW_V: &str = "Input_Low_V";
pub const PROP_INVERTED: &str = "Inverted";
pub const PROP_IS_DECADE: &str = "Is_Decade";
pub const PROP_IS_ROM: &str = "Is_Rom";
pub const PROP_ITEMTYPE: &str = "itemtype";
pub const PROP_I_OFF: &str = "I_off";
pub const PROP_I_ON: &str = "I_on";
pub const PROP_KEY: &str = "Key";
pub const PROP_KEY_LABELS: &str = "Key_Labels";
pub const PROP_LABELROT_LOWER: &str = "labelrot";
pub const PROP_LABEL_LOWER: &str = "label";
pub const PROP_LOGIC_SYMBOL: &str = "Logic_Symbol";
pub const PROP_LUX: &str = "Lux";
pub const PROP_MARGIN: &str = "Margin";
pub const PROP_MAX: &str = "Max";
pub const PROP_MAXCURRENT: &str = "MaxCurrent";
pub const PROP_MAXPULSE: &str = "MaxPulse";
pub const PROP_MAX_COUNT: &str = "Max_Count";
pub const PROP_MHZ: &str = "Mhz";
pub const PROP_MID_VOLT: &str = "Mid_Volt";
pub const PROP_MIN: &str = "Min";
pub const PROP_MINPULSE: &str = "MinPulse";
pub const PROP_MODEL: &str = "Model";
pub const PROP_MODULES: &str = "Modules";
pub const PROP_NAME: &str = "Name";
pub const PROP_NLSTEPS: &str = "NLsteps";
pub const PROP_NORM_CLOSE: &str = "Norm_Close";
pub const PROP_NUMDISPLAYS: &str = "NumDisplays";
pub const PROP_NUM_1_LAMBDA: &str = "1/Lambda";
pub const PROP_NUM_INPUTS: &str = "Num_Inputs";
pub const PROP_OFFSET: &str = "Offset";
pub const PROP_OPACITY: &str = "Opacity";
pub const PROP_OPEN_COLLECTOR: &str = "Open_Collector";
pub const PROP_OUTPUTS: &str = "Outputs";
pub const PROP_OUT_HIGH_V: &str = "Out_High_V";
pub const PROP_OUT_IMPED: &str = "Out_Imped";
pub const PROP_OUT_LOW_V: &str = "Out_Low_V";
pub const PROP_PACKAGE: &str = "Package";
pub const PROP_PD_N_LOWER: &str = "pd_n";
pub const PROP_PERIOD: &str = "Period";
pub const PROP_PGM_LOWER: &str = "pgm";
pub const PROP_PHASE: &str = "Phase";
pub const PROP_PINS: &str = "Pins";
pub const PROP_PNP: &str = "PNP";
pub const PROP_POINTLIST: &str = "pointList";
pub const PROP_POLES: &str = "Poles";
pub const PROP_PORT: &str = "Port";
pub const PROP_POS: &str = "Pos";
pub const PROP_POWER: &str = "Power";
pub const PROP_POWER_PINS: &str = "Power_Pins";
pub const PROP_PROGRAM: &str = "Program";
pub const PROP_PROGRAM_LOWER: &str = "program";
pub const PROP_P_CHANNEL: &str = "P_Channel";
pub const PROP_RDSON: &str = "RDSon";
pub const PROP_REASTEP: &str = "reaStep";
pub const PROP_RESISTANCE: &str = "Resistance";
pub const PROP_RMS: &str = "RMS";
pub const PROP_ROM: &str = "ROM";
pub const PROP_ROTATE: &str = "Rotate";
pub const PROP_ROTATION_LOWER: &str = "rotation";
pub const PROP_ROWS: &str = "Rows";
pub const PROP_RPM_NOMINAL: &str = "RPM_Nominal";
pub const PROP_RSTTIME: &str = "RstTime";
pub const PROP_RUNNING: &str = "Running";
pub const PROP_RXMAX: &str = "RxMax";
pub const PROP_RXMIN: &str = "RxMin";
pub const PROP_RYMAX: &str = "RyMax";
pub const PROP_RYMIN: &str = "RyMin";
pub const PROP_SATCURRENT: &str = "SatCurrent";
pub const PROP_SAVEPGM: &str = "savePGM";
pub const PROP_SCALE: &str = "Scale";
pub const PROP_SEGMENTS: &str = "Segments";
pub const PROP_SEMI_AMPLI: &str = "Semi_Ampli";
pub const PROP_SHOWPROP: &str = "ShowProp";
pub const PROP_SHOW_ID: &str = "Show_id";
pub const PROP_SHOW_VAL: &str = "Show_Val";
pub const PROP_SIXTEEN: &str = "Sixteen";
pub const PROP_SIZE: &str = "Size";
pub const PROP_SIZE_BYTES: &str = "Size_Bytes";
pub const PROP_SLIDER: &str = "Slider";
pub const PROP_SMALL: &str = "Small";
pub const PROP_SPEED: &str = "Speed";
pub const PROP_STARTPINID_LOWER: &str = "startpinid";
pub const PROP_STATE: &str = "State";
pub const PROP_STEP: &str = "Step";
pub const PROP_STEPS: &str = "Steps";
pub const PROP_STEPSIZE: &str = "stepSize";
pub const PROP_STEPSPS: &str = "stepsPS";
pub const PROP_STRAIN: &str = "Strain";
pub const PROP_SWITCH_PINS: &str = "Switch_Pins";
pub const PROP_T0H: &str = "T0H";
pub const PROP_T0L: &str = "T0L";
pub const PROP_T1H: &str = "T1H";
pub const PROP_T1L: &str = "T1L";
pub const PROP_TEMP: &str = "Temp";
pub const PROP_TEXT: &str = "Text";
pub const PROP_TF_PS: &str = "Tf_ps";
pub const PROP_THRESHOLD: &str = "Threshold";
pub const PROP_TIME_UPDTD: &str = "Time_Updtd";
pub const PROP_TPD_PS: &str = "Tpd_ps";
pub const PROP_TRISTATE: &str = "Tristate";
pub const PROP_TRUTH: &str = "Truth";
pub const PROP_TR_PS: &str = "Tr_ps";
pub const PROP_TUNNELS: &str = "Tunnels";
pub const PROP_UID_LOWER: &str = "uid";
pub const PROP_VALLABPOS: &str = "valLabPos";
pub const PROP_VALLABROT: &str = "valLabRot";
pub const PROP_VALUE: &str = "Value";
pub const PROP_VCRIT: &str = "Vcrit";
pub const PROP_VERTICAL_PINS: &str = "Vertical_Pins";
pub const PROP_VFLIP_LOWER: &str = "vflip";
pub const PROP_VOLTAGE: &str = "Voltage";
pub const PROP_VOLT_NEG: &str = "Volt_Neg";
pub const PROP_VOLT_NOMINAL: &str = "Volt_Nominal";
pub const PROP_VOLT_POS: &str = "Volt_Pos";
pub const PROP_VOLUME: &str = "Volume";
pub const PROP_VP: &str = "Vp";
pub const PROP_VREF: &str = "Vref";
pub const PROP_VREF_NEG: &str = "Vref_Neg";
pub const PROP_VREF_POS: &str = "Vref_Pos";
pub const PROP_V_SIZE: &str = "V_size";
pub const PROP_WAVETYPE: &str = "WaveType";
pub const PROP_WAVE_TYPE: &str = "Wave_Type";
pub const PROP_WIDTH: &str = "Width";
pub const PROP_WIDTH_LOWER: &str = "width";
pub const PROP_WIPER: &str = "Wiper";

/// C++ `parseProps`: semicolon-separated `name=value` (package / family strings).
pub fn parse_props(line: &str) -> Vec<(String, String)> {
    line.split(';')
        .filter_map(|token| {
            let token = token.trim_end();
            if token.is_empty() {
                return None;
            }
            let (raw_name, value) = match token.find('=') {
                Some(i) => (&token[..i], token[i + 1..].to_string()),
                None => (token, String::new()),
            };
            let mut name = raw_name.to_string();
            // C++ `parseProp`: only strip a leading run of spaces from the name.
            if name.starts_with(' ') {
                if let Some(i) = name.rfind(' ') {
                    name = name[i + 1..].to_string();
                }
            }
            if name.is_empty() {
                None
            } else {
                Some((name, value))
            }
        })
        .collect()
}

/// C++ `parseXmlProps`: split on `"`, name is the token after the last space
/// with the trailing `=` dropped.
pub fn parse_xml_props(line: &str) -> Vec<(String, String)> {
    let mut tokens: Vec<&str> = line.split('"').collect();
    if tokens.len() < 2 {
        return Vec::new();
    }
    tokens.pop(); // C++ `tokens.removeLast()`
    let mut properties = Vec::new();
    let mut name = String::new();
    for token in tokens {
        if name.is_empty() {
            let start = token.rfind(' ').map(|i| i + 1).unwrap_or(0);
            let mut n = token[start..].to_string();
            if n.ends_with('=') {
                n.pop();
            }
            name = n;
        } else {
            properties.push((name.clone(), token.to_string()));
            name.clear();
        }
    }
    properties
}

pub fn parse_point(val: &str) -> Option<(f64, f64)> {
    let mut parts = val.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    Some((x, y))
}

pub fn parse_point_list(val: &str) -> Vec<(f64, f64)> {
    let nums: Vec<f64> = val
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    nums.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0], c[1]))
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_sim1(src: &str) -> Result<ParsedCircuit> {
    let mut items = Vec::new();
    let mut connectors = Vec::new();
    let mut skipped = Vec::new();
    let mut analog_dt = crate::ANALOG_DT_DEFAULT;
    let mut max_nl_steps = 100_000u32;
    let mut circ = CircSettings::default();

    for raw in src.lines() {
        let line = raw.trim();
        if line.starts_with(TAG_CIRCUIT) {
            let props = parse_xml_props(line);
            if let Some(v) = prop(&props, PROP_REASTEP) {
                if let Ok(ps) = v.parse::<u64>() {
                    circ.react_step_ps = ps.max(1);
                    analog_dt = circ.analog_dt();
                }
            }
            if let Some(v) = prop(&props, PROP_NLSTEPS) {
                if let Ok(n) = v.parse::<u32>() {
                    circ.nl_steps = n.max(1);
                    max_nl_steps = circ.nl_steps;
                }
            }
            if let Some(v) = prop(&props, PROP_STEPSIZE) {
                if let Ok(n) = v.parse::<u64>() {
                    circ.step_size = n.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_STEPSPS) {
                if let Ok(n) = v.parse::<u64>() {
                    circ.steps_ps = n.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_ANIMATE_LOWER) {
                circ.animate_logic = v != "0";
            }
            if let Some(v) = prop(&props, PROP_ANICURR_LOWER) {
                circ.animate_curr = v != "0";
            }
            if let Some(v) = prop(&props, PROP_ANSI_LOWER) {
                circ.ansi = v != "0";
            }
            if let Some(v) = prop(&props, PROP_WIDTH_LOWER) {
                if let Ok(n) = v.parse::<i32>() {
                    circ.width = n.clamp(1, 10_000);
                }
            }
            if let Some(v) = prop(&props, PROP_HEIGHT_LOWER) {
                if let Ok(n) = v.parse::<i32>() {
                    circ.height = n.clamp(1, 10_000);
                }
            }
            continue;
        }
        if !line.starts_with(TAG_ITEM) {
            continue;
        }
        let mut properties = parse_xml_props(line);
        if properties.is_empty() {
            continue;
        }
        let (n0, type_) = properties.remove(0);
        if n0 != PROP_ITEMTYPE {
            continue;
        }

        match type_.as_str() {
            ITEM_CONNECTOR => {
                let mut start = None;
                let mut end = None;
                let mut id = String::new();
                let mut points = Vec::new();
                for (k, v) in properties {
                    match k.as_str() {
                        PROP_STARTPINID_LOWER => start = Some(v),
                        PROP_ENDPINID_LOWER => end = Some(v),
                        PROP_UID_LOWER | PROP_CIRCID => id = v,
                        PROP_POINTLIST => points = parse_point_list(&v),
                        _ => {}
                    }
                }
                match (start, end) {
                    (Some(s), Some(e)) => {
                        if id.is_empty() {
                            id = format!("Connector-{}", connectors.len() + 1);
                        }
                        connectors.push(ParsedConnector {
                            id,
                            start: canonicalize_legacy_pin_id(&s),
                            end: canonicalize_legacy_pin_id(&e),
                            points,
                        });
                    }
                    _ => {
                        return Err(Error::Parse(
                            "Connector missing startpinid or endpinid".into(),
                        ));
                    }
                }
            }
            ITEM_NODE => {
                let id = circ_id(&properties)?;
                items.push(parsed_item(Comp::junction(id), &properties));
            }
            ITEM_RESISTOR => {
                let id = circ_id(&properties)?;
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(crate::RESISTOR_DEFAULT_OHMS);
                items.push(parsed_item(Comp::resistor(id, r), &properties));
            }
            ITEM_BATTERY => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VOLTAGE)
                    .map(|val| parse_si(val, "V"))
                    .unwrap_or(crate::BATTERY_DEFAULT_VOLTS);
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|val| parse_si(val, "mΩ"))
                    .unwrap_or(crate::BATTERY_DEFAULT_OHMS);
                items.push(parsed_item(Comp::battery(id, v, r), &properties));
            }
            ITEM_GROUND => {
                let id = circ_id(&properties)?;
                items.push(parsed_item(Comp::ground(id), &properties));
            }
            ITEM_FIXED_VOLTAGE | ITEM_FIXEDVOLT => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VOLTAGE)
                    .map(|val| parse_si(val, "V"))
                    .unwrap_or(crate::FIXED_VOLT_DEFAULT);
                items.push(parsed_item(Comp::fixed_volt(id, v), &properties));
            }
            ITEM_CAPACITOR => {
                let id = circ_id(&properties)?;
                let c = prop(&properties, PROP_CAPACITANCE)
                    .map(|val| parse_si(val, "µF"))
                    .unwrap_or(crate::CAPACITOR_DEFAULT_FARADS);
                items.push(parsed_item(Comp::capacitor(id, c), &properties));
            }
            ITEM_ELCAPACITOR | ITEM_ELCAPACITOR_1 => {
                let id = circ_id(&properties)?;
                let c = prop(&properties, PROP_CAPACITANCE)
                    .map(|val| parse_si(val, "µF"))
                    .unwrap_or(crate::CAPACITOR_DEFAULT_FARADS);
                items.push(parsed_item(Comp::el_capacitor(id, c), &properties));
            }
            ITEM_INDUCTOR => {
                let id = circ_id(&properties)?;
                let l = prop(&properties, PROP_INDUCTANCE)
                    .map(|val| parse_si(val, "H"))
                    .unwrap_or(crate::INDUCTOR_DEFAULT_HENRIES);
                items.push(parsed_item(Comp::inductor(id, l), &properties));
            }
            ITEM_DIODE | ITEM_DIODE_LOWER => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::diode(id);
                if let Kind::Diode { ref mut state, .. } = comp.kind {
                    if let Some(v) = prop(&properties, PROP_THRESHOLD) {
                        state.threshold = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_MAXCURRENT) {
                        state.max_current = parse_si(v, "A");
                    }
                    if let Some(v) = prop(&properties, PROP_RESISTANCE) {
                        state.series_r = parse_si(v, "Ω");
                    }
                    if let Some(v) = prop(&properties, PROP_BRKDOWNV) {
                        state.bk_down = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_SATCURRENT) {
                        state.sat_cur = parse_si(v, "A");
                    }
                    if let Some(v) = prop(&properties, PROP_EMCOEF) {
                        state.em_coef = parse_si(v, "");
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_ZENER | ITEM_ZENER_LOWER | ITEM_ZENER_DIODE | ITEM_ZENER_DIODE_1 => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::zener(id);
                if let Kind::Diode { ref mut state, .. } = comp.kind {
                    if let Some(v) = prop(&properties, PROP_THRESHOLD) {
                        state.threshold = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_MAXCURRENT) {
                        state.max_current = parse_si(v, "A");
                    }
                    if let Some(v) = prop(&properties, PROP_RESISTANCE) {
                        state.series_r = parse_si(v, "Ω");
                    }
                    if let Some(v) = prop(&properties, PROP_BRKDOWNV) {
                        state.bk_down = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_SATCURRENT) {
                        state.sat_cur = parse_si(v, "A");
                    }
                    if let Some(v) = prop(&properties, PROP_EMCOEF) {
                        state.em_coef = parse_si(v, "");
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_LED | ITEM_LED_LOWER | ITEM_LED_UPPER => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::led(id);
                if let Kind::Led { ref mut state } = comp.kind {
                    if let Some(v) = prop(&properties, PROP_GROUNDED) {
                        state.grounded = parse_bool(v);
                    }
                    if let Some(v) = prop(&properties, PROP_THRESHOLD) {
                        state.threshold = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_RESISTANCE) {
                        state.impedance = parse_si(v, "Ω").max(1e-3);
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_BJT_UPPER | ITEM_BJT_LOWER | ITEM_BJT => {
                let id = circ_id(&properties)?;
                let pnp = prop(&properties, PROP_PNP).map(parse_bool).unwrap_or(false);
                let mut comp = Comp::bjt(id, pnp);
                if let Some(g) = prop(&properties, PROP_GAIN) {
                    if let Kind::Bjt { ref mut state } = comp.kind {
                        state.set_gain(parse_si(g, ""));
                    }
                }
                if let Some(v) = prop(&properties, PROP_VCRIT) {
                    if let Kind::Bjt { ref mut state } = comp.kind {
                        state.set_threshold(parse_si(v, "V"));
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_MOSFET | ITEM_MOSFET_LOWER | ITEM_MOSFET_UPPER => {
                let id = circ_id(&properties)?;
                let p_channel = prop(&properties, PROP_P_CHANNEL)
                    .map(parse_bool)
                    .unwrap_or(false);
                let depletion = prop(&properties, PROP_DEPLETION)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut comp = Comp::mosfet(id, p_channel, depletion);
                if let Some(r) = prop(&properties, PROP_RDSON) {
                    if let Kind::Mosfet { ref mut state } = comp.kind {
                        state.set_rdson(parse_si(r, "Ω"));
                    }
                }
                if let Some(th) = prop(&properties, PROP_THRESHOLD) {
                    if let Kind::Mosfet { ref mut state } = comp.kind {
                        state.set_threshold(parse_si(th, "V"));
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_JFET | ITEM_JFET_LOWER | ITEM_JFET_UPPER => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::jfet(id);
                if let Kind::Jfet { ref mut state } = comp.kind {
                    if let Some(i) = prop(&properties, PROP_IDSS) {
                        state.set_idss(parse_si(i, "A"));
                    }
                    if let Some(v) = prop(&properties, PROP_VP) {
                        state.set_vp(parse_si(v, "V"));
                    }
                    if let Some(l) = prop(&properties, PROP_NUM_1_LAMBDA) {
                        state.set_lambda_inv(parse_si(l, "V"));
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_OPAMP | ITEM_OPAMP_1 | ITEM_OPAMP_LOWER => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::opamp(id);
                if let Kind::OpAmp { ref mut state } = comp.kind {
                    if let Some(g) = prop(&properties, PROP_GAIN) {
                        state.set_gain(parse_si(g, ""));
                    }
                    if let Some(z) = prop(&properties, PROP_OUT_IMPED) {
                        state.set_out_imp(parse_si(z, "Ω"));
                    }
                    if let Some(v) = prop(&properties, PROP_VOLT_POS) {
                        state.volt_pos = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_VOLT_NEG) {
                        state.volt_neg = parse_si(v, "V");
                    }
                    if let Some(p) = prop(&properties, PROP_POWER_PINS) {
                        state.power_pins = parse_bool(p);
                    }
                    if let Some(p) = prop(&properties, PROP_SWITCH_PINS) {
                        state.switch_pins = parse_bool(p);
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_COMPARATOR | ITEM_COMPARATOR_LOWER => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::comparator(id);
                if let Kind::Comparator { ref mut state } = comp.kind {
                    apply_family_props(&properties, &mut state.family);
                    if let Some(v) = prop(&properties, PROP_OUT_HIGH_V) {
                        state.family.out_high_v = parse_si(v, "V");
                    }
                    if let Some(v) = prop(&properties, PROP_OUT_LOW_V) {
                        state.family.out_low_v = parse_si(v, "V");
                    }
                    if let Some(z) = prop(&properties, PROP_OUT_IMPED) {
                        state.set_out_imp(parse_si(z, "Ω"));
                    }
                    if let Some(p) = prop(&properties, PROP_INVERTED) {
                        state.inverted = parse_bool(p);
                    }
                    if let Some(p) = prop(&properties, PROP_OPEN_COLLECTOR) {
                        state.open_col = parse_bool(p);
                    }
                    state.apply_electric();
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_VOLTREG | ITEM_VOLTREG_LOWER | ITEM_VOLT_REGULATOR | ITEM_VOLTREGULATOR => {
                let id = circ_id(&properties)?;
                let mut comp = Comp::volt_reg(id);
                if let Kind::VoltReg { ref mut state } = comp.kind {
                    if let Some(v) = prop(&properties, PROP_VOLTAGE) {
                        state.set_out_volt(parse_si(v, "V"));
                    }
                }
                items.push(parsed_item(comp, &properties));
            }
            ITEM_PROBE => {
                let id = circ_id(&properties)?;
                let threshold = prop(&properties, PROP_THRESHOLD)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(crate::PROBE_DEFAULT_THRESHOLD);
                let small = prop(&properties, PROP_SMALL)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(Comp::probe(id, threshold, small), &properties));
            }
            ITEM_VOLTIMETER | ITEM_VOLTMETER => {
                let id = circ_id(&properties)?;
                let rms = prop(&properties, PROP_RMS).map(parse_bool).unwrap_or(false);
                items.push(parsed_item(Comp::voltmeter(id, rms), &properties));
            }
            ITEM_AMPERIMETER | ITEM_AMMETER => {
                let id = circ_id(&properties)?;
                let rms = prop(&properties, PROP_RMS).map(parse_bool).unwrap_or(false);
                items.push(parsed_item(Comp::ammeter(id, rms), &properties));
            }
            ITEM_FREQMETER | ITEM_FREQUENCIMETER => {
                let id = circ_id(&properties)?;
                let filter = prop(&properties, PROP_FILTER)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(0.1);
                items.push(parsed_item(Comp::freq_meter(id, filter), &properties));
            }
            ITEM_OSCOPE | ITEM_OSCILLOSCOPE => {
                let id = circ_id(&properties)?;
                let connect_gnd = prop(&properties, PROP_CONNECTGND)
                    .map(parse_bool)
                    .unwrap_or(true);
                let input_imped = prop(&properties, PROP_INPUTIMPED)
                    .map(|v| parse_si(v, "Ω") / 1e6)
                    .unwrap_or(10.0);
                let mut tunnels = [String::new(), String::new(), String::new(), String::new()];
                if let Some(t_str) = prop(&properties, PROP_TUNNELS) {
                    let parts: Vec<&str> = t_str.split(',').collect();
                    for (i, part) in parts.iter().enumerate().take(4) {
                        tunnels[i] = part.trim().to_string();
                    }
                }
                for i in 0..4 {
                    if let Some(t) = prop(&properties, &format!("Tunnel{}", i + 1)) {
                        tunnels[i] = t.to_string();
                    }
                }
                items.push(parsed_item(
                    Comp::oscope_full(id, connect_gnd, input_imped, tunnels),
                    &properties,
                ));
            }
            ITEM_LANALIZER | ITEM_LOGICANALYZER => {
                let id = circ_id(&properties)?;
                let connect_gnd = prop(&properties, PROP_CONNECTGND)
                    .map(parse_bool)
                    .unwrap_or(true);
                let input_imped = prop(&properties, PROP_INPUTIMPED)
                    .map(|v| parse_si(v, "Ω") / 1e6)
                    .unwrap_or(10.0);
                let mut tunnels: [String; 8] = Default::default();
                if let Some(t_str) = prop(&properties, PROP_TUNNELS) {
                    let parts: Vec<&str> = t_str.split(',').collect();
                    for (i, part) in parts.iter().enumerate().take(8) {
                        tunnels[i] = part.trim().to_string();
                    }
                }
                for i in 0..8 {
                    if let Some(t) = prop(&properties, &format!("Tunnel{}", i + 1)) {
                        tunnels[i] = t.to_string();
                    }
                }
                items.push(parsed_item(
                    Comp::lanalizer_full(id, connect_gnd, input_imped, tunnels),
                    &properties,
                ));
            }
            ITEM_AND_GATE | ITEM_ANDGATE => {
                items.push(parsed_logic_gate(&properties, LogicKind::And)?);
            }
            ITEM_OR_GATE | ITEM_ORGATE => {
                items.push(parsed_logic_gate(&properties, LogicKind::Or)?);
            }
            ITEM_XOR_GATE | ITEM_XORGATE => {
                items.push(parsed_logic_gate(&properties, LogicKind::Xor)?);
            }
            ITEM_BUFFER | ITEM_BUFFERGATE => {
                items.push(parsed_logic_gate(&properties, LogicKind::Buffer)?);
            }
            ITEM_FLIPFLOPD => {
                let id = circ_id(&properties)?;
                let mut st = crate::digital::FlipFlopState::d(&id);
                apply_family_props(&properties, &mut st.family);
                items.push(parsed_item(Comp::flipflop(id, st), &properties));
            }
            ITEM_FLIPFLOPJK => {
                let id = circ_id(&properties)?;
                let mut st = crate::digital::FlipFlopState::jk(&id);
                apply_family_props(&properties, &mut st.family);
                items.push(parsed_item(Comp::flipflop(id, st), &properties));
            }
            ITEM_FLIPFLOP_RS => {
                let id = circ_id(&properties)?;
                let mut st = crate::digital::FlipFlopState::rs(&id);
                apply_family_props(&properties, &mut st.family);
                items.push(parsed_item(Comp::flipflop(id, st), &properties));
            }
            ITEM_FLIPFLOP_T => {
                let id = circ_id(&properties)?;
                let mut st = crate::digital::FlipFlopState::t(&id);
                apply_family_props(&properties, &mut st.family);
                items.push(parsed_item(Comp::flipflop(id, st), &properties));
            }
            ITEM_LATCHD => {
                let id = circ_id(&properties)?;
                let ch = prop(&properties, PROP_CHANNELS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let mut st = crate::digital::LatchState::new(&id, ch);
                apply_family_props(&properties, &mut st.family);
                items.push(parsed_item(Comp::latch(id, st), &properties));
            }
            ITEM_TESTUNIT => {
                items.push(parsed_test_unit(&properties)?);
            }
            ITEM_MCU_UPPER | ITEM_MCU => {
                let id = circ_id(&properties)?;
                let dev_prop = prop(&properties, PROP_DEVICE_LOWER)
                    .or_else(|| prop(&properties, PROP_DEVICE))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| crate::subcircuit::device_from_id(&id));
                let spec = crate::mcu::McuItemSpec {
                    device: crate::mcu::canonicalize_device(&dev_prop),
                    frequency: prop(&properties, PROP_FREQUENCY)
                        .map(|v| parse_si(v, "MHz"))
                        .or_else(|| {
                            prop(&properties, PROP_MHZ)
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|mhz| mhz * 1e6)
                        }),
                    force_freq: prop(&properties, PROP_FORCEFREQ)
                        .map(parse_bool)
                        .unwrap_or(true),
                    program: prop(&properties, PROP_PROGRAM)
                        .or_else(|| prop(&properties, PROP_PROGRAM_LOWER))
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                    auto_load: prop(&properties, PROP_AUTO_LOAD)
                        .map(parse_bool)
                        .unwrap_or(false),
                    save_pgm: prop(&properties, PROP_SAVEPGM)
                        .map(parse_bool)
                        .unwrap_or(false),
                    pgm: prop(&properties, PROP_PGM_LOWER)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                    logic_symbol: prop(&properties, PROP_LOGIC_SYMBOL)
                        .map(parse_bool)
                        .unwrap_or(false),
                    package_name: prop(&properties, PROP_PACKAGE)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                };
                items.push(parsed_item(Comp::mcu_item(id, spec), &properties));
            }
            ITEM_PACKAGE | ITEM_SUBPACKAGE => {
                // Layout is read by `package::packages_from_sim1`; not a netlist part.
            }
            ITEM_TUNNEL => {
                let id = circ_id(&properties)?;
                let name = prop(&properties, PROP_NAME).unwrap_or("").to_string();
                let pin_id = format!("{id}-pin");
                items.push(parsed_item(Comp::tunnel(id, name, pin_id), &properties));
            }
            ITEM_SUBCIRCUIT => {
                let id = circ_id(&properties)?;
                let device = prop(&properties, PROP_DEVICE_LOWER)
                    .or_else(|| prop(&properties, PROP_DEVICE))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| crate::subcircuit::device_from_id(&id));
                let logic_symbol = prop(&properties, PROP_LOGIC_SYMBOL)
                    .map(parse_bool)
                    .unwrap_or(false);
                let package_name = prop(&properties, PROP_PACKAGE).map(|s| s.to_string());
                items.push(parsed_item(
                    Comp::subcircuit(id, device, logic_symbol, package_name),
                    &properties,
                ));
            }
            ITEM_QEMUDEVICE => {
                let id = circ_id(&properties)?;
                let device = prop(&properties, PROP_DEVICE_LOWER)
                    .or_else(|| prop(&properties, PROP_DEVICE))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| crate::subcircuit::device_from_id(&id));
                let mut q = if device.to_ascii_uppercase().starts_with("STM32") {
                    crate::qemu::QemuComp::stm32(&id, 5)
                } else {
                    crate::qemu::QemuComp::esp32(&id)
                };
                if !device.is_empty() {
                    q.device = device;
                }
                if let Some(p) = prop(&properties, PROP_PROGRAM)
                    .or_else(|| prop(&properties, PROP_PROGRAM_LOWER))
                {
                    if !p.is_empty() {
                        q.firmware = p.to_string();
                    }
                }
                if let Some(a) = prop(&properties, PROP_ARGS) {
                    q.extra_args = a.to_string();
                }
                items.push(parsed_item(Comp::qemu(id, q), &properties));
            }
            ITEM_SWITCH => {
                let id = circ_id(&properties)?;
                let closed = prop(&properties, PROP_CHECKED)
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                let nclose = prop(&properties, PROP_NORM_CLOSE)
                    .or_else(|| prop(&properties, "NormClose"))
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                let dt = prop(&properties, PROP_DOUBLE_THROW)
                    .or_else(|| prop(&properties, "DoubleThrow"))
                    .or_else(|| prop(&properties, "DT"))
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                let poles = prop(&properties, PROP_POLES)
                    .or_else(|| prop(&properties, "Poles"))
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(1);
                items.push(parsed_item(
                    Comp::switch_full(id, if nclose { !closed } else { closed }, poles, dt),
                    &properties,
                ));
            }
            ITEM_CLOCK => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VOLTAGE)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let f = prop(&properties, PROP_FREQ)
                    .map(|v| parse_si(v, "kHz"))
                    .unwrap_or(1.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Clock {
                            voltage: v,
                            freq_khz: f,
                            state: false,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_RAIL => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VOLTAGE)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Rail { voltage: v },
                    },
                    &properties,
                ));
            }
            ITEM_WAVEGEN => {
                let id = circ_id(&properties)?;
                let wave_type = prop(&properties, PROP_WAVE_TYPE)
                    .or_else(|| prop(&properties, PROP_WAVETYPE))
                    .unwrap_or("Sine")
                    .to_string();
                let f = prop(&properties, PROP_FREQUENCY)
                    .or_else(|| prop(&properties, PROP_FREQ))
                    .map(|v| parse_si(v, "Hz"))
                    .unwrap_or(1000.0);
                let a = prop(&properties, PROP_AMPLITUDE)
                    .or_else(|| prop(&properties, PROP_SEMI_AMPLI))
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let o = prop(&properties, PROP_OFFSET)
                    .or_else(|| prop(&properties, PROP_MID_VOLT))
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(0.0);
                let d = prop(&properties, PROP_DUTY)
                    .map(|v| {
                        let parsed = parse_si(v, "%");
                        if parsed > 1.0 { parsed / 100.0 } else { parsed }
                    })
                    .unwrap_or(0.5);
                let phase = prop(&properties, PROP_PHASE)
                    .map(|v| parse_si(v, "°"))
                    .unwrap_or(0.0);
                let steps = prop(&properties, PROP_STEPS)
                    .and_then(|v| v.trim().parse::<i32>().ok())
                    .unwrap_or(100);
                let bipolar = prop(&properties, PROP_BIPOLAR)
                    .map(parse_bool)
                    .unwrap_or(false);
                let floating = prop(&properties, PROP_FLOATING)
                    .map(parse_bool)
                    .unwrap_or(false);
                let file = prop(&properties, PROP_FILE).unwrap_or("").to_string();
                items.push(parsed_item(
                    Comp::wave_gen_with_file(
                        id, wave_type, f, a, o, d, phase, steps, bipolar, floating, file,
                    ),
                    &properties,
                ));
            }
            ITEM_VOLTAGE_SOURCE | ITEM_VOLTSOURCE => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VALUE)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let running = prop(&properties, PROP_RUNNING)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::VoltSource { value: v, running },
                    },
                    &properties,
                ));
            }
            ITEM_CURRENT_SOURCE | ITEM_CURRSOURCE => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VALUE)
                    .map(|v| parse_si(v, "A"))
                    .unwrap_or(0.01);
                let running = prop(&properties, PROP_RUNNING)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::CurrSource { value: v, running },
                    },
                    &properties,
                ));
            }
            ITEM_CSOURCE => {
                let id = circ_id(&properties)?;
                let gain = prop(&properties, PROP_GAIN)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Csource {
                            control_pins: true,
                            curr_source: false,
                            curr_control: false,
                            gain,
                            volt: 0.0,
                            current: 0.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_PUSH => {
                let id = circ_id(&properties)?;
                let poles = prop(&properties, PROP_POLES)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let norm_close = prop(&properties, PROP_NORM_CLOSE)
                    .map(parse_bool)
                    .unwrap_or(false);
                let closed = norm_close;
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Push { closed, poles },
                    },
                    &properties,
                ));
            }
            ITEM_SWITCHDIP => {
                let id = circ_id(&properties)?;
                let size = prop(&properties, PROP_SIZE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let state = prop(&properties, PROP_STATE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let common_pin = prop(&properties, PROP_COMMON_PIN)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::SwitchDip {
                            size,
                            state,
                            common_pin,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_RELAY => {
                let id = circ_id(&properties)?;
                let norm_close = prop(&properties, PROP_NORM_CLOSE)
                    .map(parse_bool)
                    .unwrap_or(false);
                let double_throw = prop(&properties, PROP_DOUBLE_THROW)
                    .map(parse_bool)
                    .unwrap_or(false);
                let poles = prop(&properties, PROP_POLES)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let i_on = prop(&properties, PROP_I_ON)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.02);
                let i_off = prop(&properties, PROP_I_OFF)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.01);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Relay {
                            norm_close,
                            double_throw,
                            poles,
                            i_on,
                            i_off,
                            active: false,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_KEYPAD => {
                let id = circ_id(&properties)?;
                let rows = prop(&properties, PROP_ROWS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let cols = prop(&properties, PROP_COLS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let key = prop(&properties, PROP_KEY_LABELS)
                    .or_else(|| prop(&properties, PROP_KEY))
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let diodes = prop(&properties, PROP_DIODES)
                    .map(parse_bool)
                    .unwrap_or(false);
                let dir = prop(&properties, PROP_DIR).map(parse_bool).unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::KeyPad {
                            rows,
                            cols,
                            key,
                            diodes,
                            dir,
                            pressed: None,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_POTENTIOMETER | ITEM_POTENTIOMETER_LOWER => {
                let id = circ_id(&properties)?;
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(1_000.0);
                let wiper = prop(&properties, PROP_WIPER)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.5);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Potentiometer {
                            resistance: r,
                            wiper,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_TOUCHPAD | ITEM_TOUCHPAD_LOWER | ITEM_TOUCHPADR => {
                let id = circ_id(&properties)?;
                let width = prop(&properties, PROP_WIDTH)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(240);
                let height = prop(&properties, PROP_HEIGHT)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(320);
                let rx_min = prop(&properties, PROP_RXMIN)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(100.0);
                let rx_max = prop(&properties, PROP_RXMAX)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(500.0);
                let ry_min = prop(&properties, PROP_RYMIN)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(100.0);
                let ry_max = prop(&properties, PROP_RYMAX)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(500.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::TouchPad {
                            width,
                            height,
                            rx_min,
                            rx_max,
                            ry_min,
                            ry_max,
                            x_pos: -1,
                            y_pos: -1,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_KY023_UPPER | ITEM_KY023_LOWER | ITEM_KY_023_UPPER => {
                let id = circ_id(&properties)?;
                items.push(parsed_item(Comp::ky023(id, 0.0, 0.0, false), &properties));
            }
            ITEM_KY040_UPPER | ITEM_KY040_LOWER | ITEM_KY_040_UPPER => {
                let id = circ_id(&properties)?;
                let steps = prop(&properties, PROP_STEPS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(20);
                items.push(parsed_item(
                    Comp::ky040(id, steps, 1, false, false, false),
                    &properties,
                ));
            }
            ITEM_SR04_UPPER | ITEM_SR04_LOWER | ITEM_HC_SR04_UPPER => {
                let id = circ_id(&properties)?;
                let dist = prop(&properties, PROP_DISTANCE)
                    .map(|v| parse_si(v, "m"))
                    .unwrap_or(0.5);
                let slider = prop(&properties, PROP_SLIDER)
                    .map(parse_bool)
                    .unwrap_or(true);
                items.push(parsed_item(Comp::sr04(id, dist, slider), &properties));
            }
            ITEM_DHT22_UPPER | ITEM_DHT22_LOWER | ITEM_DHT22 | ITEM_DHT11_UPPER
            | ITEM_DHT11_LOWER | ITEM_DHT11 => {
                let id = circ_id(&properties)?;
                let model = prop(&properties, PROP_MODEL).unwrap_or("DHT22");
                let temp = prop(&properties, PROP_TEMP)
                    .map(|v| parse_si(v, "°C"))
                    .unwrap_or(22.5);
                let humi = prop(&properties, PROP_HUMI)
                    .map(|v| parse_si(v, "%"))
                    .unwrap_or(68.5);
                items.push(parsed_item(Comp::dht22(id, model, temp, humi), &properties));
            }
            ITEM_DS18B20_UPPER | ITEM_DS18B20_LOWER | ITEM_DS18B20 => {
                let id = circ_id(&properties)?;
                let rom = prop(&properties, PROP_ROM).unwrap_or("28FF2B450000");
                let temp = prop(&properties, PROP_TEMP)
                    .map(|v| parse_si(v, "°C"))
                    .unwrap_or(25.0);
                items.push(parsed_item(Comp::ds18b20(id, rom, temp), &properties));
            }
            ITEM_DS1621_UPPER | ITEM_DS1621_LOWER => {
                let id = circ_id(&properties)?;
                let temp = prop(&properties, PROP_TEMP)
                    .map(|v| parse_si(v, "°C"))
                    .unwrap_or(25.0);
                items.push(parsed_item(Comp::ds1621(id, temp), &properties));
            }
            ITEM_DS1307_UPPER | ITEM_DS1307_LOWER => {
                let id = circ_id(&properties)?;
                let time_updated = prop(&properties, PROP_TIME_UPDTD)
                    .map(parse_bool)
                    .unwrap_or(true);
                items.push(parsed_item(Comp::ds1307(id, time_updated), &properties));
            }
            ITEM_DCMOTOR | ITEM_DCMOTOR_LOWER => {
                let id = circ_id(&properties)?;
                let rpm = prop(&properties, PROP_RPM_NOMINAL)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                let volt = prop(&properties, PROP_VOLT_NOMINAL)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(100.0);
                items.push(parsed_item(Comp::dcmotor(id, rpm, volt, r), &properties));
            }
            ITEM_STEPPER | ITEM_STEPPER_LOWER => {
                let id = circ_id(&properties)?;
                let bipolar = prop(&properties, PROP_BIPOLAR)
                    .map(parse_bool)
                    .unwrap_or(false);
                let steps = prop(&properties, PROP_STEPS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(32);
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(100.0);
                items.push(parsed_item(
                    Comp::stepper(id, bipolar, steps, r),
                    &properties,
                ));
            }
            ITEM_SERVO | ITEM_SERVO_LOWER => {
                let id = circ_id(&properties)?;
                let speed = prop(&properties, PROP_SPEED)
                    .map(|v| parse_si(v, "s/60°"))
                    .unwrap_or(0.2);
                let min_p = prop(&properties, PROP_MINPULSE)
                    .map(|v| parse_si(v, "µs"))
                    .unwrap_or(1000.0);
                let max_p = prop(&properties, PROP_MAXPULSE)
                    .map(|v| parse_si(v, "µs"))
                    .unwrap_or(2000.0);
                items.push(parsed_item(
                    Comp::servo(id, speed, min_p, max_p),
                    &properties,
                ));
            }
            ITEM_SDCARD | ITEM_SDCARD_LOWER => {
                let id = circ_id(&properties)?;
                let file = prop(&properties, PROP_FILE).unwrap_or("");
                items.push(parsed_item(Comp::sdcard(id, file), &properties));
            }
            ITEM_ESP01 | ITEM_ESP01_LOWER => {
                let id = circ_id(&properties)?;
                let baud = prop(&properties, PROP_BAUDRATE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(115200);
                let debug = prop(&properties, PROP_DEBUG)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(Comp::esp01(id, baud, debug), &properties));
            }
            ITEM_TFTDISPLAY
            | ITEM_TFT_DISPLAY_LOWER
            | ITEM_ILI9341_UPPER
            | ITEM_ST7789_UPPER
            | ITEM_ST7735_UPPER
            | ITEM_GC9A01A_UPPER => {
                let id = circ_id(&properties)?;
                let controller = prop(&properties, PROP_CONTROLLER).unwrap_or("ST7735");
                let width = prop(&properties, PROP_WIDTH)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(128);
                let height = prop(&properties, PROP_HEIGHT)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(160);
                let scale = prop(&properties, PROP_SCALE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                let bgr = prop(&properties, PROP_BGR).map(parse_bool).unwrap_or(false);
                items.push(parsed_item(
                    Comp::tft_display(id, controller, width, height, scale, bgr),
                    &properties,
                ));
            }
            ITEM_PCD8544_UPPER | ITEM_PCD8544_LOWER | ITEM_NOKIA5110 => {
                let id = circ_id(&properties)?;
                let contrast = prop(&properties, PROP_CONTRAST)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(50);
                let bias = prop(&properties, PROP_BIAS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                items.push(parsed_item(Comp::pcd8544(id, contrast, bias), &properties));
            }
            ITEM_SH1107_UPPER | ITEM_SH1107_LOWER => {
                let id = circ_id(&properties)?;
                let width = prop(&properties, PROP_WIDTH)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(64);
                let height = prop(&properties, PROP_HEIGHT)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(128);
                items.push(parsed_item(Comp::sh1107(id, width, height), &properties));
            }
            ITEM_KS0108_UPPER | ITEM_KS0108_LOWER => {
                let id = circ_id(&properties)?;
                items.push(parsed_item(Comp::ks0108(id, 128, 64), &properties));
            }
            ITEM_PCF8833_UPPER | ITEM_PCF8833_LOWER => {
                let id = circ_id(&properties)?;
                items.push(parsed_item(Comp::pcf8833(id, 132, 132), &properties));
            }
            ITEM_AIP31068_UPPER | ITEM_AIP31068_LOWER => {
                let id = circ_id(&properties)?;
                let rows = prop(&properties, PROP_ROWS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2);
                let cols = prop(&properties, PROP_COLS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(16);
                items.push(parsed_item(Comp::aip31068(id, rows, cols), &properties));
            }
            ITEM_VARRESISTOR | ITEM_VAR_RESISTOR_LOWER => {
                let id = circ_id(&properties)?;
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(1_000.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::VarResistor { resistance: r },
                    },
                    &properties,
                ));
            }
            ITEM_RESISTORDIP | ITEM_RESISTOR_DIP_LOWER => {
                let id = circ_id(&properties)?;
                let size = prop(&properties, PROP_SIZE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let r = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(100.0);
                let bussed = prop(&properties, PROP_BUSSED)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::ResistorDip {
                            size,
                            resistance: r,
                            bussed,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_LDR_UPPER | ITEM_LDR_LOWER => {
                let id = circ_id(&properties)?;
                let lux = prop(&properties, PROP_LUX)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Ldr {
                            resistance: 1000.0,
                            lux,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_THERMISTOR | ITEM_THERMISTOR_LOWER => {
                let id = circ_id(&properties)?;
                let temp = prop(&properties, PROP_TEMP)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(25.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Thermistor {
                            resistance: 10_000.0,
                            temp_c: temp,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_RTD_UPPER | ITEM_RTD_LOWER => {
                let id = circ_id(&properties)?;
                let temp = prop(&properties, PROP_TEMP)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(25.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Rtd {
                            resistance: 100.0,
                            temp_c: temp,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_STRAIN | ITEM_STRAIN_LOWER => {
                let id = circ_id(&properties)?;
                let strain = prop(&properties, PROP_STRAIN)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Strain {
                            resistance: 350.0,
                            strain,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_VARCAPACITOR | ITEM_VAR_CAPACITOR_LOWER => {
                let id = circ_id(&properties)?;
                let c = prop(&properties, PROP_CAPACITANCE)
                    .map(|v| parse_si(v, "F"))
                    .unwrap_or(100e-12);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::VarCapacitor {
                            capacitance: c,
                            volt: 0.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_VARINDUCTOR | ITEM_VAR_INDUCTOR_LOWER => {
                let id = circ_id(&properties)?;
                let l = prop(&properties, PROP_INDUCTANCE)
                    .map(|v| parse_si(v, "H"))
                    .unwrap_or(1e-3);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::VarInductor {
                            inductance: l,
                            ieq: 0.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_TRANSFORMER | ITEM_TRANSFORMER_LOWER => {
                let id = circ_id(&properties)?;
                let l1 = prop(&properties, PROP_INDUCTANCE1)
                    .map(|v| parse_si(v, "H"))
                    .unwrap_or(1.0);
                let l2 = prop(&properties, PROP_INDUCTANCE2)
                    .map(|v| parse_si(v, "H"))
                    .unwrap_or(1.0);
                let k = prop(&properties, PROP_COUPLING)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.99);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Transformer {
                            inductance1: l1,
                            inductance2: l2,
                            coupling: k,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_SCR_UPPER | ITEM_SCR_LOWER => {
                let id = circ_id(&properties)?;
                let v_gate_th = prop(&properties, PROP_GATE_TH)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(0.7);
                let i_hold = prop(&properties, PROP_HOLDCURR)
                    .map(|v| parse_si(v, "A"))
                    .unwrap_or(0.0082);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Scr {
                            v_gate_th,
                            i_hold,
                            conducting: false,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_DIAC | ITEM_DIAC_LOWER | ITEM_DIAC_UPPER => {
                let id = circ_id(&properties)?;
                let v_breakover = prop(&properties, PROP_BREAKOVER)
                    .or_else(|| prop(&properties, PROP_BRKVOLT))
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(30.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Diac {
                            v_breakover,
                            conducting: false,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_TRIAC | ITEM_TRIAC_LOWER | ITEM_TRIAC_UPPER => {
                let id = circ_id(&properties)?;
                let v_gate_th = prop(&properties, PROP_GATE_TH)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(0.7);
                let i_hold = prop(&properties, PROP_HOLDCURR)
                    .map(|v| parse_si(v, "A"))
                    .unwrap_or(0.0082);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Triac {
                            v_gate_th,
                            i_hold,
                            conducting: false,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_ANALOGMUX
            | ITEM_MUXANALOG
            | ITEM_MUX_ANALOG_LOWER
            | ITEM_ANALOG_MUX_LOWER
            | ITEM_ANALOG_MUX => {
                let id = circ_id(&properties)?;
                let ch = prop(&properties, PROP_CHANNELS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::AnalogMux {
                            channels: ch,
                            selected: 0,
                            on_res: 100.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_RGBLED => {
                let id = circ_id(&properties)?;
                let ca = prop(&properties, PROP_COMMON_ANODE)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::RgbLed {
                            common_anode: ca,
                            state_r: 0.0,
                            state_g: 0.0,
                            state_b: 0.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_LEDBAR => {
                let id = circ_id(&properties)?;
                let seg = prop(&properties, PROP_SEGMENTS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);
                let grounded = prop(&properties, PROP_GROUNDED)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::LedBar {
                            segments: seg,
                            states: 0,
                            grounded,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_SEVENSEGMENT => {
                let id = circ_id(&properties)?;
                let ca = prop(&properties, PROP_COMMON_ANODE)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::SevenSegment {
                            common_anode: ca,
                            segments: 0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_LEDMATRIX => {
                let id = circ_id(&properties)?;
                let rows = prop(&properties, PROP_ROWS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let cols = prop(&properties, PROP_COLS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let vertical_pins = prop(&properties, PROP_VERTICAL_PINS)
                    .map(parse_bool)
                    .unwrap_or(false);
                let color = prop(&properties, PROP_COLOR).unwrap_or("Red").to_string();
                let threshold = prop(&properties, PROP_THRESHOLD)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(2.0);
                let max_current = prop(&properties, PROP_MAXCURRENT)
                    .map(|v| parse_si(v, "A"))
                    .unwrap_or(0.03);
                let resistance = prop(&properties, PROP_RESISTANCE)
                    .map(|v| parse_si(v, "Ω"))
                    .unwrap_or(0.6);
                items.push(parsed_item(
                    Comp::led_matrix(
                        id,
                        rows,
                        cols,
                        vertical_pins,
                        color,
                        threshold,
                        max_current,
                        resistance,
                    ),
                    &properties,
                ));
            }
            ITEM_MAX72XX => {
                let id = circ_id(&properties)?;
                let modules = prop(&properties, PROP_MODULES)
                    .or_else(|| prop(&properties, PROP_NUMDISPLAYS))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let color = prop(&properties, PROP_COLOR).unwrap_or("Red").to_string();
                items.push(parsed_item(Comp::max72xx(id, modules, color), &properties));
            }
            ITEM_WS2812_UPPER | ITEM_WS2812 => {
                let id = circ_id(&properties)?;
                let count = prop(&properties, PROP_COUNT)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let rows = prop(&properties, PROP_ROWS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let cols = prop(&properties, PROP_COLS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(count);
                let rst_time_ns = prop(&properties, PROP_RSTTIME)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(50000);
                let t0h_ns = prop(&properties, PROP_T0H)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(400);
                let t0l_ns = prop(&properties, PROP_T0L)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(850);
                let t1h_ns = prop(&properties, PROP_T1H)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(800);
                let t1l_ns = prop(&properties, PROP_T1L)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(450);
                items.push(parsed_item(
                    Comp::ws2812(
                        id,
                        count,
                        rows,
                        cols,
                        rst_time_ns,
                        t0h_ns,
                        t0l_ns,
                        t1h_ns,
                        t1l_ns,
                    ),
                    &properties,
                ));
            }
            ITEM_HD44780 => {
                let id = circ_id(&properties)?;
                let rows = prop(&properties, PROP_ROWS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2);
                let cols = prop(&properties, PROP_COLS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(16);
                items.push(parsed_item(Comp::hd44780(id, rows, cols), &properties));
            }
            ITEM_SSD1306 => {
                let id = circ_id(&properties)?;
                let color = prop(&properties, PROP_COLOR).unwrap_or("White");
                let width = prop(&properties, PROP_WIDTH)
                    .map(|v| v.trim_end_matches("_px").trim())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(128);
                let height = prop(&properties, PROP_HEIGHT)
                    .map(|v| v.trim_end_matches("_px").trim())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(64);
                let rotate = prop(&properties, PROP_ROTATE)
                    .map(parse_bool)
                    .unwrap_or(true);
                let control_code = prop(&properties, PROP_CONTROL_CODE)
                    .or_else(|| prop(&properties, "ControlCode"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                let freq_khz = prop(&properties, PROP_FREQUENCY)
                    .map(|v| {
                        let clean = v.trim().trim_end_matches("_kHz").trim();
                        if let Ok(f) = clean.parse::<f64>() {
                            f
                        } else {
                            parse_si(v, "Hz") / 1e3
                        }
                    })
                    .unwrap_or(100.0);
                items.push(parsed_item(
                    Comp::ssd1306(id, width, height, control_code, color, rotate, freq_khz),
                    &properties,
                ));
            }
            ITEM_AUDIOOUT => {
                let id = circ_id(&properties)?;
                let imp = prop(&properties, PROP_IMPEDANCE)
                    .map(|val| parse_si(val, "Ω"))
                    .unwrap_or(8.0);
                let vol = prop(&properties, PROP_VOLUME)
                    .and_then(|v| v.trim_end_matches('%').trim().parse().ok())
                    .unwrap_or(100.0);
                let freq = prop(&properties, PROP_FREQUENCY)
                    .map(|val| parse_si(val, "Hz"))
                    .unwrap_or(1000.0);
                let buzzer = prop(&properties, PROP_BUZZER)
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                let source = crate::audio::Source::new(imp, vol, freq, buzzer);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::AudioOut {
                            source,
                            last_v: 0.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_LAMP => {
                let id = circ_id(&properties)?;
                let v = prop(&properties, PROP_VOLTAGE)
                    .map(|val| parse_si(val, "V"))
                    .unwrap_or(12.0);
                let p = prop(&properties, PROP_POWER)
                    .and_then(|val| val.parse().ok())
                    .unwrap_or(24.0);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Lamp {
                            voltage: v,
                            power: p,
                            r_cold: 6.0,
                            resistance: 6.0,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_BUS => {
                let id = circ_id(&properties)?;
                let w = prop(&properties, PROP_WIDTH)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Bus { width: w },
                    },
                    &properties,
                ));
            }
            ITEM_SOCKET => {
                let id = circ_id(&properties)?;
                let p = prop(&properties, PROP_PINS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Socket { pins_count: p },
                    },
                    &properties,
                ));
            }
            ITEM_HEADER => {
                let id = circ_id(&properties)?;
                let p = prop(&properties, PROP_PINS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::Header { pins_count: p },
                    },
                    &properties,
                ));
            }
            ITEM_SERIALPORT => {
                let id = circ_id(&properties)?;
                let port = prop(&properties, PROP_PORT)
                    .unwrap_or("/dev/ttyUSB0")
                    .to_string();
                let baud = prop(&properties, PROP_BAUD)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(9600);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::SerialPort {
                            port_name: port,
                            baud_rate: baud,
                        },
                    },
                    &properties,
                ));
            }
            ITEM_SERIALTERM => {
                let id = circ_id(&properties)?;
                let baud = prop(&properties, PROP_BAUD)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(9600);
                items.push(parsed_item(
                    Comp {
                        id: id.clone(),
                        kind: Kind::SerialTerm { baud_rate: baud },
                    },
                    &properties,
                ));
            }
            ITEM_DIAL => {
                let id = circ_id(&properties)?;
                let value = prop(&properties, PROP_VALUE)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                let min_val = prop(&properties, PROP_MIN)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                let max_val = prop(&properties, PROP_MAX)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100.0);
                let step = prop(&properties, PROP_STEP)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                items.push(parsed_item(
                    Comp::dial(id, value, min_val, max_val, step),
                    &properties,
                ));
            }
            ITEM_MUX => {
                let id = circ_id(&properties)?;
                let addr_bits = prop(&properties, PROP_ADDR_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2);
                let mut m = crate::digital::MuxState::new(&id, addr_bits);
                apply_family_props(&properties, &mut m.family);
                m.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Mux(m),
                    },
                    &properties,
                ));
            }
            ITEM_DEMUX => {
                let id = circ_id(&properties)?;
                let addr_bits = prop(&properties, PROP_ADDR_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2);
                let inverted = prop(&properties, PROP_INVERTED)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut d = crate::digital::DemuxState::new(&id, addr_bits, inverted);
                apply_family_props(&properties, &mut d.family);
                d.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Demux(d),
                    },
                    &properties,
                ));
            }
            ITEM_BCDTODEC => {
                let id = circ_id(&properties)?;
                let sixteen = prop(&properties, PROP_SIXTEEN)
                    .map(parse_bool)
                    .unwrap_or(false);
                let active_low = prop(&properties, PROP_ACTIVE_LOW)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut b = crate::digital::BcdToDecState::new(&id, sixteen, active_low);
                apply_family_props(&properties, &mut b.family);
                b.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::BcdToDec(b),
                    },
                    &properties,
                ));
            }
            ITEM_DECTOBCD => {
                let id = circ_id(&properties)?;
                let sixteen = prop(&properties, PROP_SIXTEEN)
                    .map(parse_bool)
                    .unwrap_or(false);
                let active_low = prop(&properties, PROP_ACTIVE_LOW)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut d = crate::digital::DecToBcdState::new(&id, sixteen, active_low);
                apply_family_props(&properties, &mut d.family);
                d.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::DecToBcd(d),
                    },
                    &properties,
                ));
            }
            ITEM_BCDTO7S => {
                let id = circ_id(&properties)?;
                let common_anode = prop(&properties, PROP_COMMON_ANODE)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut b = crate::digital::BcdTo7SState::new(&id, common_anode);
                apply_family_props(&properties, &mut b.family);
                b.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::BcdTo7S(b),
                    },
                    &properties,
                ));
            }
            ITEM_SEVENSEGMENTBCD => {
                let id = circ_id(&properties)?;
                let color = prop(&properties, PROP_COLOR).unwrap_or("Red").to_string();
                let common_anode = prop(&properties, PROP_COMMON_ANODE)
                    .map(parse_bool)
                    .unwrap_or(false);
                items.push(parsed_item(
                    Comp::seven_segment_bcd(id, color, common_anode),
                    &properties,
                ));
            }
            ITEM_I2CTOPARALLEL => {
                let id = circ_id(&properties)?;
                let address = prop(&properties, PROP_ADDRESS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0x20);
                let mut p = crate::digital::I2CToParallelState::new(&id, address);
                apply_family_props(&properties, &mut p.family);
                p.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::I2CToParallel(p),
                    },
                    &properties,
                ));
            }
            ITEM_ADC_UPPER | ITEM_ADC => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let vref_pos = prop(&properties, PROP_VREF_POS)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let vref_neg = prop(&properties, PROP_VREF_NEG)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(0.0);
                let mut a = crate::digital::AdcState::new(&id, bits, vref_pos, vref_neg);
                apply_family_props(&properties, &mut a.family);
                a.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Adc(a),
                    },
                    &properties,
                ));
            }
            ITEM_DAC_UPPER | ITEM_DAC => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let vref = prop(&properties, PROP_VREF)
                    .map(|v| parse_si(v, "V"))
                    .unwrap_or(5.0);
                let mut d = crate::digital::DacState::new(&id, bits, vref);
                apply_family_props(&properties, &mut d.family);
                d.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Dac(d),
                    },
                    &properties,
                ));
            }
            ITEM_COUNTER => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let max_count = prop(&properties, PROP_MAX_COUNT)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or((1 << bits) - 1);
                let mut c = crate::digital::CounterState::new(&id, bits, max_count);
                apply_family_props(&properties, &mut c.family);
                c.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Counter(c),
                    },
                    &properties,
                ));
            }
            ITEM_BINCOUNTER => {
                let id = circ_id(&properties)?;
                let is_decade = prop(&properties, PROP_IS_DECADE)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut b = crate::digital::BinCounterState::new(&id, is_decade);
                apply_family_props(&properties, &mut b.family);
                b.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::BinCounter(b),
                    },
                    &properties,
                ));
            }
            ITEM_FULLADDER => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let mut fa = crate::digital::FullAdderState::new(&id, bits);
                apply_family_props(&properties, &mut fa.family);
                fa.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::FullAdder(fa),
                    },
                    &properties,
                ));
            }
            ITEM_MAGNITUDECOMP => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let mut mc = crate::digital::MagnitudeCompState::new(&id, bits);
                apply_family_props(&properties, &mut mc.family);
                mc.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::MagnitudeComp(mc),
                    },
                    &properties,
                ));
            }
            ITEM_SHIFTREG => {
                let id = circ_id(&properties)?;
                let bits = prop(&properties, PROP_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let mut sr = crate::digital::ShiftRegState::new(&id, bits);
                apply_family_props(&properties, &mut sr.family);
                sr.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::ShiftReg(sr),
                    },
                    &properties,
                ));
            }
            ITEM_FUNCTION => {
                let id = circ_id(&properties)?;
                let inputs = prop(&properties, PROP_INPUTS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3);
                let expr = prop(&properties, PROP_EXPRESSION).unwrap_or("").to_string();
                let mut f = crate::digital::FunctionState::new(&id, inputs, &expr);
                apply_family_props(&properties, &mut f.family);
                f.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Function(f),
                    },
                    &properties,
                ));
            }
            ITEM_MEMORY => {
                let id = circ_id(&properties)?;
                let addr_bits = prop(&properties, PROP_ADDR_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let data_bits = prop(&properties, PROP_DATA_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let is_rom = prop(&properties, PROP_IS_ROM)
                    .map(parse_bool)
                    .unwrap_or(false);
                let mut m = crate::digital::MemoryState::new(&id, addr_bits, data_bits, is_rom);
                apply_family_props(&properties, &mut m.family);
                m.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Memory(m),
                    },
                    &properties,
                ));
            }
            ITEM_DYNAMICMEMORY => {
                let id = circ_id(&properties)?;
                let addr_bits = prop(&properties, PROP_ADDR_BITS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let mut dm = crate::digital::DynamicMemoryState::new(&id, addr_bits);
                apply_family_props(&properties, &mut dm.family);
                dm.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::DynamicMemory(dm),
                    },
                    &properties,
                ));
            }
            ITEM_I2CRAM => {
                let id = circ_id(&properties)?;
                let size_bytes = prop(&properties, PROP_SIZE_BYTES)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(256);
                let dev_address = prop(&properties, PROP_DEV_ADDRESS)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0x50);
                let mut r = crate::digital::I2CRamState::new(&id, size_bytes, dev_address);
                apply_family_props(&properties, &mut r.family);
                r.apply_family();
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::I2CRam(r),
                    },
                    &properties,
                ));
            }
            ITEM_LM555 | ITEM_LM555_UPPER | ITEM_NUM_555 => {
                let id = circ_id(&properties)?;
                let lm = crate::digital::Lm555State::new(&id);
                items.push(parsed_item(
                    Comp {
                        id,
                        kind: Kind::Lm555(lm),
                    },
                    &properties,
                ));
            }
            ITEM_RECTANGLE | ITEM_ELLIPSE | ITEM_LINE | ITEM_TEXT | ITEM_TEXTCOMPONENT
            | ITEM_IMAGE => {
                let id = circ_id(&properties).unwrap_or_else(|_| "Shape-1".into());
                let width = prop(&properties, PROP_H_SIZE)
                    .or_else(|| prop(&properties, PROP_WIDTH))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(if type_ == ITEM_IMAGE { 80.0 } else { 50.0 });
                let height = prop(&properties, PROP_V_SIZE)
                    .or_else(|| prop(&properties, PROP_HEIGHT))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(if type_ == ITEM_IMAGE { 80.0 } else { 30.0 });
                let text = prop(&properties, PROP_TEXT)
                    .unwrap_or(if type_ == ITEM_TEXT || type_ == ITEM_TEXTCOMPONENT {
                        "... TEXT ..."
                    } else {
                        ""
                    })
                    .replace("&#xa;", "\n")
                    .replace("&#x22;", "\"")
                    .replace("&#x3D;", "=")
                    .replace("&#x3C;", "<")
                    .replace("&#x3E;", ">");
                let color = prop(&properties, PROP_COLOR)
                    .unwrap_or(if type_ == ITEM_TEXT || type_ == ITEM_TEXTCOMPONENT {
                        "#ffffdc"
                    } else {
                        "#808080"
                    })
                    .to_string();
                let font = prop(&properties, PROP_FONT)
                    .unwrap_or("SansSerif")
                    .to_string();
                let font_color = prop(&properties, "FontColor")
                    .or_else(|| prop(&properties, PROP_FONT_COLOR))
                    .unwrap_or("#000000")
                    .to_string();
                let font_size = prop(&properties, "FontSize")
                    .or_else(|| prop(&properties, PROP_FONT_SIZE))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);
                let border = prop(&properties, PROP_BORDER)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(if type_ == ITEM_TEXT || type_ == ITEM_TEXTCOMPONENT {
                        1
                    } else if type_ == ITEM_IMAGE {
                        0
                    } else {
                        2
                    });
                let opacity = prop(&properties, PROP_OPACITY)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                items.push(parsed_item(
                    Comp::shape(
                        id, type_, width, height, text, color, font, font_color, font_size, border,
                        opacity,
                    ),
                    &properties,
                ));
            }
            other => skipped.push(other.to_string()),
        }
    }

    Ok(ParsedCircuit {
        items,
        connectors,
        skipped,
        analog_dt,
        max_nl_steps,
        circ,
    })
}

enum LogicKind {
    And,
    Or,
    Xor,
    Buffer,
}

fn parsed_logic_gate(properties: &[(String, String)], kind: LogicKind) -> Result<ParsedItem> {
    let id = circ_id(properties)?;
    let n = prop(properties, PROP_NUM_INPUTS)
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let mut st = match kind {
        LogicKind::And => crate::digital::GateState::and(&id, n),
        LogicKind::Or => crate::digital::GateState::or(&id, n),
        LogicKind::Xor => crate::digital::GateState::xor(&id, n),
        LogicKind::Buffer => crate::digital::GateState::buffer(&id),
    };
    apply_family_props(properties, &mut st.family);
    st.apply_family();
    if let Some(v) = prop(properties, PROP_INITHIGH) {
        st.init_high = parse_bool(v);
    }
    if let Some(v) = prop(properties, PROP_OPEN_COLLECTOR) {
        st.set_open_col(parse_bool(v));
    }
    if let Some(v) = prop(properties, PROP_INVERTED) {
        st.output.set_inverted(parse_bool(v));
    }
    if let Some(v) = prop(properties, PROP_TRISTATE) {
        st.set_tristate(parse_bool(v));
    }
    Ok(parsed_item(Comp::gate(id, st), properties))
}

fn parsed_test_unit(properties: &[(String, String)]) -> Result<ParsedItem> {
    let id = circ_id(properties)?;
    let mut st = crate::digital::TestUnitState::new(&id);
    apply_family_props(properties, &mut st.family);
    if let Some(v) = prop(properties, PROP_INPUTS) {
        st.set_inputs(&id, v);
    }
    if let Some(v) = prop(properties, PROP_OUTPUTS) {
        st.set_outputs(&id, v);
    }
    st.apply_family();
    if let Some(v) = prop(properties, PROP_PERIOD) {
        st.set_period(parse_si(v, "ns"));
    }
    if let Some(v) = prop(properties, PROP_TRUTH) {
        st.set_truth_str(v);
    }
    Ok(parsed_item(Comp::test_unit_with(id, st), properties))
}

fn apply_family_props(properties: &[(String, String)], family: &mut crate::digital::LogicFamily) {
    if let Some(v) = prop(properties, PROP_TPD_PS) {
        family.set_prop_delay_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, PROP_PD_N_LOWER) {
        family.delay_mult = parse_si(v, "");
    }
    if let Some(v) = prop(properties, PROP_TR_PS) {
        family.set_rise_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, PROP_TF_PS) {
        family.set_fall_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, PROP_OUT_HIGH_V) {
        family.out_high_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, PROP_OUT_LOW_V) {
        family.out_low_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, PROP_OUT_IMPED) {
        family.out_imp = parse_si(v, "Ω").max(1e-14);
    }
    if let Some(v) = prop(properties, PROP_INPUT_HIGH_V) {
        family.inp_high_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, PROP_INPUT_LOW_V) {
        family.inp_low_v = parse_si(v, "V");
    }
}

fn parsed_item(comp: Comp, properties: &[(String, String)]) -> ParsedItem {
    ParsedItem {
        comp,
        pos: prop(properties, PROP_POS).and_then(parse_point),
        rotation: prop(properties, PROP_ROTATION_LOWER)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        hflip: parse_flip(prop(properties, PROP_HFLIP_LOWER)),
        vflip: parse_flip(prop(properties, PROP_VFLIP_LOWER)),
        label: prop(properties, PROP_LABEL_LOWER).map(|s| s.to_string()),
        show_id: prop(properties, PROP_SHOW_ID)
            .map(parse_bool)
            .unwrap_or(false),
        label_pos: prop(properties, PROP_IDLABPOS).and_then(parse_point),
        label_rot: prop(properties, PROP_LABELROT_LOWER).and_then(|v| v.parse().ok()),
        show_val: prop(properties, PROP_SHOW_VAL)
            .map(parse_bool)
            .unwrap_or(false),
        show_prop: prop(properties, PROP_SHOWPROP).map(|s| s.to_string()),
        val_pos: prop(properties, PROP_VALLABPOS).and_then(parse_point),
        val_rot: prop(properties, PROP_VALLABROT).and_then(|v| v.parse().ok()),
    }
}

fn parse_flip(v: Option<&str>) -> i32 {
    match v.and_then(|s| s.parse::<i32>().ok()) {
        Some(-1) => -1,
        _ => 1,
    }
}

fn circ_id(properties: &[(String, String)]) -> Result<String> {
    prop(properties, PROP_CIRCID)
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Parse("item missing CircId".into()))
}

pub fn prop<'a>(properties: &'a [(String, String)], name: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

pub fn parse_bool(v: &str) -> bool {
    v == "true" || v == "1"
}

/// Format a number that C++ `QString::number` would emit for a grid point.
pub fn fmt_coord(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v}")
    }
}

pub fn fmt_pos(x: f64, y: f64) -> String {
    format!("{},{}", fmt_coord(x), fmt_coord(y))
}

pub fn fmt_point_list(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .flat_map(|(x, y)| [fmt_coord(*x), fmt_coord(*y)])
        .collect::<Vec<_>>()
        .join(",")
}

#[allow(dead_code)]
pub fn fmt_resistance(ohms: f64) -> String {
    format_si(ohms, "Ω")
}

#[allow(dead_code)]
pub fn fmt_voltage(volts: f64) -> String {
    format_si(volts, "V")
}

/// Legacy XML aliases mapping old item names / casing to canonical TYPE_IDs.
pub const LEGACY_ITEMTYPE_ALIASES: &[(&str, &str)] = &[
    ("opAmp", crate::components::OpAmp::TYPE_ID),
    ("Voltimeter", crate::components::Voltmeter::TYPE_ID),
    ("Amperimeter", crate::components::Ammeter::TYPE_ID),
    ("LAnalizer", crate::components::LogicAnalyzer::TYPE_ID),
    ("Fixed Voltage", crate::components::FixedVolt::TYPE_ID),
    ("And Gate", crate::components::AndGate::TYPE_ID),
    ("Or Gate", crate::components::OrGate::TYPE_ID),
    ("Xor Gate", crate::components::XorGate::TYPE_ID),
    ("Not Gate", crate::components::NotGate::TYPE_ID),
    ("elCapacitor", crate::components::ElCapacitor::TYPE_ID),
    ("Frequencimeter", crate::components::FreqMeter::TYPE_ID),
    ("Voltage Source", crate::components::VoltSource::TYPE_ID),
    ("Current Source", crate::components::CurrSource::TYPE_ID),
    ("KY-023", crate::components::KY023::TYPE_ID),
    ("ky023", crate::components::KY023::TYPE_ID),
    ("Ky023", crate::components::KY023::TYPE_ID),
    ("KY-040", crate::components::KY040::TYPE_ID),
    ("ky040", crate::components::KY040::TYPE_ID),
    ("Ky040", crate::components::KY040::TYPE_ID),
    ("Oscilloscope", crate::components::Oscope::TYPE_ID),
    ("Volt. Regulator", crate::components::VoltReg::TYPE_ID),
    ("Buffer Gate", crate::components::BufferGate::TYPE_ID),
    ("Buffer", crate::components::BufferGate::TYPE_ID),
    ("BcdTo7S", crate::components::BcdTo7Segment::TYPE_ID),
    ("LatchD", crate::components::Latch::TYPE_ID),
    ("FlipFlopD", crate::components::FlipFlop::TYPE_ID),
    ("FlipFlopJK", crate::components::FlipFlop::TYPE_ID),
    ("FlipFlop RS", crate::components::FlipFlop::TYPE_ID),
    ("FlipFlop T", crate::components::FlipFlop::TYPE_ID),
    ("BJT", crate::components::Bjt::TYPE_ID),
    ("touchpad", crate::components::TouchPad::TYPE_ID),
    ("TouchPadR", crate::components::TouchPad::TYPE_ID),
    ("HC-SR04", crate::components::SR04::TYPE_ID),
    ("sr04", crate::components::SR04::TYPE_ID),
    ("Sr04", crate::components::SR04::TYPE_ID),
    ("dht22", crate::components::DHT22::TYPE_ID),
    ("Dht22", crate::components::DHT22::TYPE_ID),
    ("DHT11", crate::components::DHT22::TYPE_ID),
    ("dht11", crate::components::DHT22::TYPE_ID),
    ("Dht11", crate::components::DHT22::TYPE_ID),
    ("ds18b20", crate::components::DS18B20::TYPE_ID),
    ("Ds18b20", crate::components::DS18B20::TYPE_ID),
    ("ds1621", crate::components::DS1621::TYPE_ID),
    ("Ds1621", crate::components::DS1621::TYPE_ID),
    ("ds1307", crate::components::DS1307::TYPE_ID),
    ("Ds1307", crate::components::DS1307::TYPE_ID),
    ("dcmotor", crate::components::DcMotor::TYPE_ID),
    ("DC Motor", crate::components::DcMotor::TYPE_ID),
    ("stepper", crate::components::Stepper::TYPE_ID),
    ("Stepper Motor", crate::components::Stepper::TYPE_ID),
    ("servo", crate::components::Servo::TYPE_ID),
    ("Servo Motor", crate::components::Servo::TYPE_ID),
    ("sdcard", crate::components::SdCard::TYPE_ID),
    ("SDCard", crate::components::SdCard::TYPE_ID),
    ("ESP-01", crate::components::Esp01::TYPE_ID),
    ("esp01", crate::components::Esp01::TYPE_ID),
    ("ESP01", crate::components::Esp01::TYPE_ID),
    ("TFTDisplay", crate::components::TftDisplay::TYPE_ID),
    ("tft_display", crate::components::TftDisplay::TYPE_ID),
    ("TFT Display", crate::components::TftDisplay::TYPE_ID),
    ("ILI9341", crate::components::TftDisplay::TYPE_ID),
    ("ST7789", crate::components::TftDisplay::TYPE_ID),
    ("ST7735", crate::components::TftDisplay::TYPE_ID),
    ("GC9A01A", crate::components::TftDisplay::TYPE_ID),
    ("PCD8544", crate::components::Pcd8544::TYPE_ID),
    ("pcd8544", crate::components::Pcd8544::TYPE_ID),
    ("Nokia5110", crate::components::Pcd8544::TYPE_ID),
    ("SH1107", crate::components::Sh1107::TYPE_ID),
    ("sh1107", crate::components::Sh1107::TYPE_ID),
    ("KS0108", crate::components::Ks0108::TYPE_ID),
    ("ks0108", crate::components::Ks0108::TYPE_ID),
    ("PCF8833", crate::components::Pcf8833::TYPE_ID),
    ("pcf8833", crate::components::Pcf8833::TYPE_ID),
    ("AIP31068", crate::components::Aip31068::TYPE_ID),
    ("aip31068", crate::components::Aip31068::TYPE_ID),
    ("HD44780", crate::components::Hd44780::TYPE_ID),
    ("hd44780", crate::components::Hd44780::TYPE_ID),
    ("SSD1306", crate::components::Ssd1306::TYPE_ID),
    ("ssd1306", crate::components::Ssd1306::TYPE_ID),
    ("7Segment", crate::components::SevenSegment::TYPE_ID),
    ("seven_segment", crate::components::SevenSegment::TYPE_ID),
    ("Seven Segment", crate::components::SevenSegment::TYPE_ID),
    ("7 Segment", crate::components::SevenSegment::TYPE_ID),
    (
        "SevenSegmentBcd",
        crate::components::SevenSegmentBCD::TYPE_ID,
    ),
    ("7SegmentBCD", crate::components::SevenSegmentBCD::TYPE_ID),
    ("7Segment BCD", crate::components::SevenSegmentBCD::TYPE_ID),
    (
        "SevenSegment BCD",
        crate::components::SevenSegmentBCD::TYPE_ID,
    ),
    (
        "seven_segment_bcd",
        crate::components::SevenSegmentBCD::TYPE_ID,
    ),
    ("Led Matrix", crate::components::LedMatrix::TYPE_ID),
    ("led_matrix", crate::components::LedMatrix::TYPE_ID),
    ("LED Matrix", crate::components::LedMatrix::TYPE_ID),
    ("MAX7219", crate::components::Max72xx::TYPE_ID),
    ("max7219", crate::components::Max72xx::TYPE_ID),
    ("MAX72XX", crate::components::Max72xx::TYPE_ID),
    ("max72xx", crate::components::Max72xx::TYPE_ID),
    ("Max7219", crate::components::Max72xx::TYPE_ID),
    ("Led Bar", crate::components::LedBar::TYPE_ID),
    ("LED Bar", crate::components::LedBar::TYPE_ID),
    ("led_bar", crate::components::LedBar::TYPE_ID),
    ("RGB LED", crate::components::RgbLed::TYPE_ID),
    ("RGBLed", crate::components::RgbLed::TYPE_ID),
    ("rgb_led", crate::components::RgbLed::TYPE_ID),
    ("rgbLed", crate::components::RgbLed::TYPE_ID),
    ("RGB Led", crate::components::RgbLed::TYPE_ID),
    ("WS2812", crate::components::Ws2812::TYPE_ID),
    ("ws2812", crate::components::Ws2812::TYPE_ID),
    ("WS2812B", crate::components::Ws2812::TYPE_ID),
    ("ws2812b", crate::components::Ws2812::TYPE_ID),
    ("74xx161", crate::components::Counter::TYPE_ID),
    ("74xx163", crate::components::Counter::TYPE_ID),
    ("BinaryCounter", crate::components::Counter::TYPE_ID),
    ("binary_counter", crate::components::Counter::TYPE_ID),
    ("counter", crate::components::Counter::TYPE_ID),
    ("bin_counter", crate::components::BinCounter::TYPE_ID),
    ("bincounter", crate::components::BinCounter::TYPE_ID),
    ("74xx90", crate::components::BinCounter::TYPE_ID),
    ("shift_reg", crate::components::ShiftReg::TYPE_ID),
    ("shiftreg", crate::components::ShiftReg::TYPE_ID),
    ("ShiftRegister", crate::components::ShiftReg::TYPE_ID),
    ("74xx164", crate::components::ShiftReg::TYPE_ID),
    ("74xx165", crate::components::ShiftReg::TYPE_ID),
    ("74xx595", crate::components::ShiftReg::TYPE_ID),
    ("magnitude_comp", crate::components::MagnitudeComp::TYPE_ID),
    ("magnitudecomp", crate::components::MagnitudeComp::TYPE_ID),
    (
        "MagnitudeComparator",
        crate::components::MagnitudeComp::TYPE_ID,
    ),
    ("74xx85", crate::components::MagnitudeComp::TYPE_ID),
    ("ADC", crate::components::Adc::TYPE_ID),
    ("adc", crate::components::Adc::TYPE_ID),
    ("DAC", crate::components::Dac::TYPE_ID),
    ("dac", crate::components::Dac::TYPE_ID),
    ("i2c_to_parallel", crate::components::I2CToParallel::TYPE_ID),
    ("I2C_To_Parallel", crate::components::I2CToParallel::TYPE_ID),
    ("PCF8574", crate::components::I2CToParallel::TYPE_ID),
    ("pcf8574", crate::components::I2CToParallel::TYPE_ID),
    ("i2ctoparallel", crate::components::I2CToParallel::TYPE_ID),
    ("LM555", crate::components::Lm555::TYPE_ID),
    ("555", crate::components::Lm555::TYPE_ID),
    ("lm555", crate::components::Lm555::TYPE_ID),
    ("NE555", crate::components::Lm555::TYPE_ID),
    ("ne555", crate::components::Lm555::TYPE_ID),
    ("RAM", crate::components::Memory::TYPE_ID),
    ("ROM", crate::components::Memory::TYPE_ID),
    ("ram", crate::components::Memory::TYPE_ID),
    ("rom", crate::components::Memory::TYPE_ID),
    ("memory", crate::components::Memory::TYPE_ID),
    ("dynamic_memory", crate::components::DynamicMemory::TYPE_ID),
    ("DRAM", crate::components::DynamicMemory::TYPE_ID),
    ("dram", crate::components::DynamicMemory::TYPE_ID),
    ("dynamicmemory", crate::components::DynamicMemory::TYPE_ID),
    ("I2C_RAM", crate::components::I2CRam::TYPE_ID),
    ("i2c_ram", crate::components::I2CRam::TYPE_ID),
    ("i2cram", crate::components::I2CRam::TYPE_ID),
    ("I2CRAM", crate::components::I2CRam::TYPE_ID),
    ("24C04", crate::components::I2CRam::TYPE_ID),
    ("24c04", crate::components::I2CRam::TYPE_ID),
    ("function", crate::components::Function::TYPE_ID),
    ("Func", crate::components::Function::TYPE_ID),
    ("func", crate::components::Function::TYPE_ID),
    ("transformer", crate::components::Transformer::TYPE_ID),
    ("TRANSFORMER", crate::components::Transformer::TYPE_ID),
    ("SCR", crate::components::Scr::TYPE_ID),
    ("scr", crate::components::Scr::TYPE_ID),
    ("DIAC", crate::components::Diac::TYPE_ID),
    ("diac", crate::components::Diac::TYPE_ID),
    ("TRIAC", crate::components::Triac::TYPE_ID),
    ("triac", crate::components::Triac::TYPE_ID),
    ("csource", crate::components::Csource::TYPE_ID),
    ("CSOURCE", crate::components::Csource::TYPE_ID),
    ("CSource", crate::components::Csource::TYPE_ID),
    ("Controlled Source", crate::components::Csource::TYPE_ID),
    ("controlled_source", crate::components::Csource::TYPE_ID),
    ("resistor_dip", crate::components::ResistorDip::TYPE_ID),
    ("Resistor_Dip", crate::components::ResistorDip::TYPE_ID),
    ("resistordip", crate::components::ResistorDip::TYPE_ID),
    ("RESISTOR_DIP", crate::components::ResistorDip::TYPE_ID),
    ("Resistor DIP", crate::components::ResistorDip::TYPE_ID),
    ("LDR", crate::components::Ldr::TYPE_ID),
    ("ldr", crate::components::Ldr::TYPE_ID),
    ("THERMISTOR", crate::components::Thermistor::TYPE_ID),
    ("thermistor", crate::components::Thermistor::TYPE_ID),
    ("RTD", crate::components::Rtd::TYPE_ID),
    ("rtd", crate::components::Rtd::TYPE_ID),
    ("STRAIN", crate::components::Strain::TYPE_ID),
    ("strain", crate::components::Strain::TYPE_ID),
    ("Audio_Out", crate::components::AudioOut::TYPE_ID),
    ("Audio Out", crate::components::AudioOut::TYPE_ID),
    ("audio_out", crate::components::AudioOut::TYPE_ID),
    ("audioout", crate::components::AudioOut::TYPE_ID),
    ("Buzzer", crate::components::AudioOut::TYPE_ID),
    ("buzzer", crate::components::AudioOut::TYPE_ID),
    ("Speaker", crate::components::AudioOut::TYPE_ID),
    ("speaker", crate::components::AudioOut::TYPE_ID),
    ("LAMP", crate::components::Lamp::TYPE_ID),
    ("lamp", crate::components::Lamp::TYPE_ID),
    ("oscope", crate::components::Oscope::TYPE_ID),
    ("OSCOPE", crate::components::Oscope::TYPE_ID),
    ("wavegen", crate::components::WaveGen::TYPE_ID),
    ("Wave_Gen", crate::components::WaveGen::TYPE_ID),
    ("Wave Generator", crate::components::WaveGen::TYPE_ID),
    ("wave_gen", crate::components::WaveGen::TYPE_ID),
    ("WAVEGEN", crate::components::WaveGen::TYPE_ID),
    ("Wave_Generator", crate::components::WaveGen::TYPE_ID),
    ("WaveGenerator", crate::components::WaveGen::TYPE_ID),
    ("tunnel", crate::components::Tunnel::TYPE_ID),
    ("TUNNEL", crate::components::Tunnel::TYPE_ID),
    ("bus", crate::components::Bus::TYPE_ID),
    ("BUS", crate::components::Bus::TYPE_ID),
    ("socket", crate::components::Socket::TYPE_ID),
    ("SOCKET", crate::components::Socket::TYPE_ID),
    ("header", crate::components::Header::TYPE_ID),
    ("HEADER", crate::components::Header::TYPE_ID),
    ("PinHeader", crate::components::Header::TYPE_ID),
    ("pin_header", crate::components::Header::TYPE_ID),
    ("Pin_Header", crate::components::Header::TYPE_ID),
    ("pinheader", crate::components::Header::TYPE_ID),
    ("PIN_HEADER", crate::components::Header::TYPE_ID),
    ("serial_port", crate::components::SerialPort::TYPE_ID),
    ("Serial_Port", crate::components::SerialPort::TYPE_ID),
    ("serialport", crate::components::SerialPort::TYPE_ID),
    ("SERIAL_PORT", crate::components::SerialPort::TYPE_ID),
    ("SERIALPORT", crate::components::SerialPort::TYPE_ID),
    ("Serial Port", crate::components::SerialPort::TYPE_ID),
    ("serial_term", crate::components::SerialTerm::TYPE_ID),
    ("Serial_Term", crate::components::SerialTerm::TYPE_ID),
    ("serialterm", crate::components::SerialTerm::TYPE_ID),
    ("SERIAL_TERM", crate::components::SerialTerm::TYPE_ID),
    ("SERIALTERM", crate::components::SerialTerm::TYPE_ID),
    ("Serial Term", crate::components::SerialTerm::TYPE_ID),
    ("Terminal", crate::components::SerialTerm::TYPE_ID),
    ("terminal", crate::components::SerialTerm::TYPE_ID),
    ("TERMINAL", crate::components::SerialTerm::TYPE_ID),
    ("subcircuit", crate::components::Subcircuit::TYPE_ID),
    ("SUBCIRCUIT", crate::components::Subcircuit::TYPE_ID),
    ("SubCircuit", crate::components::Subcircuit::TYPE_ID),
    ("Sub_Circuit", crate::components::Subcircuit::TYPE_ID),
    ("sub_circuit", crate::components::Subcircuit::TYPE_ID),
    ("subpackage", crate::components::SubPackage::TYPE_ID),
    ("SUBPACKAGE", crate::components::SubPackage::TYPE_ID),
    ("Sub_Package", crate::components::SubPackage::TYPE_ID),
    ("sub_package", crate::components::SubPackage::TYPE_ID),
    ("Subcircuit Package", crate::components::SubPackage::TYPE_ID),
    ("subcircuit_package", crate::components::SubPackage::TYPE_ID),
    ("Package", crate::components::SubPackage::TYPE_ID),
    ("package", crate::components::SubPackage::TYPE_ID),
    ("PACKAGE", crate::components::SubPackage::TYPE_ID),
    ("dial", crate::components::Dial::TYPE_ID),
    ("DIAL", crate::components::Dial::TYPE_ID),
    ("shape", crate::components::Shape::TYPE_ID),
    ("SHAPE", crate::components::Shape::TYPE_ID),
    ("Rectangle", crate::components::Shape::TYPE_ID),
    ("rectangle", crate::components::Shape::TYPE_ID),
    ("RECTANGLE", crate::components::Shape::TYPE_ID),
    ("Ellipse", crate::components::Shape::TYPE_ID),
    ("ellipse", crate::components::Shape::TYPE_ID),
    ("ELLIPSE", crate::components::Shape::TYPE_ID),
    ("Line", crate::components::Shape::TYPE_ID),
    ("line", crate::components::Shape::TYPE_ID),
    ("LINE", crate::components::Shape::TYPE_ID),
    ("Text", crate::components::Shape::TYPE_ID),
    ("text", crate::components::Shape::TYPE_ID),
    ("TEXT", crate::components::Shape::TYPE_ID),
    ("TextComponent", crate::components::Shape::TYPE_ID),
    ("Image", crate::components::Shape::TYPE_ID),
    ("image", crate::components::Shape::TYPE_ID),
    ("IMAGE", crate::components::Shape::TYPE_ID),
    ("mcu", crate::components::Mcu::TYPE_ID),
    ("MCU", crate::components::Mcu::TYPE_ID),
    ("Micro", crate::components::Mcu::TYPE_ID),
    ("micro", crate::components::Mcu::TYPE_ID),
    ("MICRO", crate::components::Mcu::TYPE_ID),
    ("Microcontroller", crate::components::Mcu::TYPE_ID),
    ("microcontroller", crate::components::Mcu::TYPE_ID),
    ("MICROCONTROLLER", crate::components::Mcu::TYPE_ID),
    ("qemu", crate::components::QemuDevice::TYPE_ID),
    ("QEMU", crate::components::QemuDevice::TYPE_ID),
    ("qemu_device", crate::components::QemuDevice::TYPE_ID),
    ("Qemu_Device", crate::components::QemuDevice::TYPE_ID),
    ("qemudevice", crate::components::QemuDevice::TYPE_ID),
    ("QEMUDEVICE", crate::components::QemuDevice::TYPE_ID),
    ("Qemu", crate::components::QemuDevice::TYPE_ID),
];

pub fn resolve_legacy_itemtype(itemtype: &str) -> Option<&'static str> {
    LEGACY_ITEMTYPE_ALIASES
        .iter()
        .find(|(alias, _)| *alias == itemtype)
        .map(|(_, canonical)| *canonical)
}

/// Remap legacy pin IDs from .sim1 / .sim2 XML files to canonical pin IDs.
pub fn canonicalize_legacy_pin_id(pin_id: &str) -> String {
    let Some(last_dash) = pin_id.rfind('-') else {
        return pin_id.to_string();
    };
    let prefix = &pin_id[..last_dash];
    let suffix = &pin_id[last_dash + 1..];

    let canonical_suffix = match suffix {
        "VRX" | "vrx" => "vrx",
        "VRY" | "vry" => "vry",
        "SW" | "sw" => "sw",
        "CLK" | "clk" => "clk",
        "DT" | "dt" => "dt",
        "Trig" | "trigpin" => "trigpin",
        "Echo" | "outpin" => "outpin",
        "Vcc" | "vccpin" | "vccPin" => "vccpin",
        "gndpin" | "gndPin" | "gdnPin" | "PinGnd" => "gndpin",
        "In" | "inpin" => "inpin",
        "Data" | "inPin" => "inPin",
        "NC" | "ncPin" => "ncPin",
        "Vdd" | "vddPin" | "PinVdd" => "PinVdd",
        "SDA" | "inPin0" => "inPin0",
        "SCL" | "inPin1" => "inPin1",
        "Tout" | "outPin0" => "outPin0",
        "A0" | "inPin2" => "inPin2",
        "A1" | "inPin3" => "inPin3",
        "A2" | "inPin4" => "inPin4",
        "PinSDA" => "PinSDA",
        "PinSCL" => "PinSCL",
        "SQW" | "PinSQW" => "PinSQW",
        "PinA" | "aPin" | "mt1Pin" | "+" => PIN_LEFT,
        "PinB" | "kPin" | "mt2Pin" | "-" => PIN_RIGHT,
        "PinM" => PIN_POT_W,
        "PinInput" => "com",
        "PinEnable" => "en",
        "XP" | "vrx_p" => "vrx_p",
        "XM" | "vrx_m" => "vrx_m",
        "YP" | "vry_p" => "vry_p",
        "YM" | "vry_m" => "vry_m",
        "A+" | "PinA1" => "PinA1",
        "B+" | "PinB1" => "PinB1",
        "Co" | "PinCo" => "PinCo",
        "A-" | "PinA2" => "PinA2",
        "B-" | "PinB2" => "PinB2",
        "V+" | "PinV+" => "PinV+",
        "Sig" | "PinSig" => "PinSig",
        "CS" | "PinCS" => "PinCS",
        "DI" | "PinDI" => "PinDI",
        "CK" | "PinCK" => "PinCK",
        "DO" | "PinDO" => "PinDO",
        "Tx" | "pin0" => "pin0",
        "Rx" | "pin1" => "pin1",
        s if s.starts_with("pinY") => {
            return format!("{prefix}-ch{}", &s[4..]);
        }
        s if s.starts_with("pinP") => {
            return format!("{prefix}-lPin{}", &s[4..]);
        }
        s if s.starts_with("switch") && s.ends_with("pinN") => {
            return format!("{prefix}-rPin{}", &s[6..s.len() - 4]);
        }
        _ => suffix,
    };

    format!("{prefix}-{canonical_suffix}")
}

fn parse_legacy_graphic_attrs(attrs: &[(String, String)]) -> GraphicAttrs {
    let mut g = GraphicAttrs::default();
    for (k, v) in attrs {
        match k.as_str() {
            "Pos" | "pos" => g.pos = parse_point(v),
            "rotation" | "Rotate" | "rot" => g.rotation = v.parse().unwrap_or(0.0),
            "hflip" => g.hflip = v.parse().unwrap_or(1),
            "vflip" => g.vflip = v.parse().unwrap_or(1),
            "label" => g.label = Some(v.clone()),
            "Show_id" | "ShowId" | "show_id" => g.show_id = parse_bool(v),
            "idLabPos" | "id_lab_pos" => g.label_pos = parse_point(v),
            "labelrot" | "label_rot" => g.label_rot = v.parse().unwrap_or(0),
            "Show_Val" | "ShowVal" | "show_val" => g.show_val = parse_bool(v),
            "ShowProp" | "show_prop" => g.show_prop = Some(v.clone()),
            "valLabPos" | "val_lab_pos" => g.val_pos = parse_point(v),
            "valLabRot" | "val_rot" => g.val_rot = v.parse().unwrap_or(0),
            _ => {}
        }
    }
    g
}

fn normalize_legacy_attrs(attrs: &[(String, String)]) -> Vec<(String, String)> {
    attrs
        .iter()
        .map(|(k, v)| {
            let norm_k = match k.as_str() {
                "Show_id" | "ShowId" => "ShowId",
                "Show_Val" | "ShowVal" => "ShowVal",
                "ShowProp" | "show_prop" => "ShowProp",
                "Auto_Load" | "AutoLoad" => "AutoLoad",
                "savePGM" | "SavePgm" => "SavePgm",
                "saveEPR" | "SaveEepr" => "SaveEepr",
                "Active_Low" | "ActiveLow" => "ActiveLow",
                "Norm_Close" | "NormClose" => "NormClose",
                "DT" | "Double_Throw" | "DoubleThrow" => "DoubleThrow",
                "Poles" | "poles" => "Poles",
                _ => k.as_str(),
            };
            (norm_k.to_string(), v.clone())
        })
        .collect()
}

/// Parse a legacy `.sim1` / `.sim2` circuit document and convert it to modern [`ParsedCirc1`].
pub fn parse_legacy_to_circ1(src: &str) -> Result<ParsedCirc1> {
    let mut circ = CircSettings::default();
    let mut items = Vec::new();
    let mut connectors = Vec::new();
    let mut saw_circuit = false;

    for raw in src.lines() {
        let line = raw.trim();
        if line.starts_with(TAG_CIRCUIT) {
            saw_circuit = true;
            let props = parse_xml_props(line);
            if let Some(v) = prop(&props, PROP_REASTEP) {
                if let Ok(ps) = v.parse::<u64>() {
                    circ.react_step_ps = ps.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_NLSTEPS) {
                if let Ok(n) = v.parse::<u32>() {
                    circ.nl_steps = n.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_STEPSIZE) {
                if let Ok(n) = v.parse::<u64>() {
                    circ.step_size = n.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_STEPSPS) {
                if let Ok(n) = v.parse::<u64>() {
                    circ.steps_ps = n.max(1);
                }
            }
            if let Some(v) = prop(&props, PROP_ANIMATE_LOWER) {
                circ.animate_logic = v != "0";
            }
            if let Some(v) = prop(&props, PROP_ANICURR_LOWER) {
                circ.animate_curr = v != "0";
            }
            if let Some(v) = prop(&props, PROP_ANSI_LOWER) {
                circ.ansi = v != "0";
            }
            if let Some(v) = prop(&props, PROP_WIDTH_LOWER) {
                if let Ok(n) = v.parse::<i32>() {
                    circ.width = n.clamp(1, 10_000);
                }
            }
            if let Some(v) = prop(&props, PROP_HEIGHT_LOWER) {
                if let Ok(n) = v.parse::<i32>() {
                    circ.height = n.clamp(1, 10_000);
                }
            }
            continue;
        }
        if !line.starts_with(TAG_ITEM) {
            continue;
        }
        let mut properties = parse_xml_props(line);
        if properties.is_empty() {
            continue;
        }
        let (n0, type_) = properties.remove(0);
        if n0 != PROP_ITEMTYPE {
            continue;
        }
        match type_.as_str() {
            ITEM_CONNECTOR => {
                let mut start = None;
                let mut end = None;
                let mut id = String::new();
                let mut points = Vec::new();
                for (k, v) in properties {
                    match k.as_str() {
                        PROP_STARTPINID_LOWER | "start" => start = Some(v),
                        PROP_ENDPINID_LOWER | "end" => end = Some(v),
                        PROP_UID_LOWER | PROP_CIRCID | "id" => id = v,
                        PROP_POINTLIST => points = parse_point_list(&v),
                        _ => {}
                    }
                }
                match (start, end) {
                    (Some(s), Some(e)) => {
                        if id.is_empty() {
                            id = format!("Connector-{}", connectors.len() + 1);
                        }
                        connectors.push(ParsedConnector {
                            id,
                            start: canonicalize_legacy_pin_id(&s),
                            end: canonicalize_legacy_pin_id(&e),
                            points,
                        });
                    }
                    _ => {
                        return Err(Error::Parse(
                            "Connector missing startpinid or endpinid".into(),
                        ));
                    }
                }
            }
            other => {
                let circ_id = prop(&properties, PROP_CIRCID)
                    .or_else(|| prop(&properties, PROP_UID_LOWER))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{other}-{}", items.len() + 1));
                let canonical_type = resolve_legacy_itemtype(other).unwrap_or(other);
                let graphic = parse_legacy_graphic_attrs(&properties);
                let normalized_attrs = normalize_legacy_attrs(&properties);
                let part =
                    crate::circ1::create_part(canonical_type, circ_id.clone(), &normalized_attrs)?;
                items.push(ParsedCirc1Item {
                    itemtype: canonical_type.to_string(),
                    circ_id,
                    attrs: normalized_attrs,
                    graphic,
                    part,
                });
            }
        }
    }

    if !saw_circuit {
        return Err(Error::Parse("missing <circuit> element".into()));
    }

    Ok(ParsedCirc1 {
        version: crate::circ1::FORMAT_VERSION.to_string(),
        circ,
        items,
        connectors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_props() {
        let p = parse_xml_props(
            r#"<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />"#,
        );
        assert_eq!(p[0], ("itemtype".into(), "Resistor".into()));
        assert_eq!(p[1], ("CircId".into(), "Resistor-1".into()));
        assert_eq!(p[2], ("Resistance".into(), "1 kΩ".into()));
    }

    #[test]
    fn semicolon_props() {
        let p = parse_props("Pin; type=; xpos=-8; ypos=8; id=in");
        assert_eq!(p[0], ("Pin".into(), String::new()));
        assert_eq!(p[1], ("type".into(), String::new()));
        assert_eq!(p[2], ("xpos".into(), "-8".into()));
        assert_eq!(p[4], ("id".into(), "in".into()));
    }

    #[test]
    fn point_list() {
        assert_eq!(
            parse_point_list("100,-100,148,-100"),
            vec![(100.0, -100.0), (148.0, -100.0)]
        );
        assert_eq!(parse_point("200,-100"), Some((200.0, -100.0)));
    }

    #[test]
    fn parse_bjt_and_mosfet() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="BJT" CircId="BJT-1" PNP="true" Gain="200" Pos="0,0" />
<item itemtype="Mosfet" CircId="Mosfet-1" P_Channel="true" Depletion="true" RDSon="2 Ω" Threshold="1 V" Pos="40,0" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 2);
        match &p.items[0].comp.kind {
            Kind::Bjt { state } => {
                assert!(state.pnp);
                assert!((state.gain - 200.0).abs() < 1e-9);
            }
            other => panic!("expected BJT, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::Mosfet { state } => {
                assert!(state.p_channel);
                assert!(state.depletion);
                assert!((state.rdson - 2.0).abs() < 1e-9);
                assert!((state.threshold - 1.0).abs() < 1e-9);
            }
            other => panic!("expected Mosfet, got {other:?}"),
        }
    }

    #[test]
    fn parse_opamp() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="opAmp" CircId="opAmp-1" Gain="500" Out_Imped="1 Ω" Volt_Pos="12 V" Volt_Neg="-12 V" Power_Pins="true" Switch_Pins="true" Pos="8,4" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::OpAmp { state } => {
                assert!((state.gain - 500.0).abs() < 1e-9);
                assert!((state.out_imp - 1.0).abs() < 1e-9);
                assert!((state.volt_pos - 12.0).abs() < 1e-9);
                assert!((state.volt_neg + 12.0).abs() < 1e-9);
                assert!(state.power_pins);
                assert!(state.switch_pins);
            }
            other => panic!("expected opAmp, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_jfet() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Jfet" CircId="Jfet-1" Idss="20 mA" Vp="-2 V" 1/Lambda="500 V" Pos="16,8" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Jfet { state } => {
                assert!((state.idss - 0.020).abs() < 1e-9);
                assert!((state.vp + 2.0).abs() < 1e-9);
                assert!((state.lambda_inv - 500.0).abs() < 1e-9);
            }
            other => panic!("expected Jfet, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((16.0, 8.0)));
    }

    #[test]
    fn parse_comparator() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Comparator" CircId="Comparator-1" Out_High_V="3.3 V" Out_Low_V="0.2 V" Out_Imped="20 Ω" Inverted="true" Open_Collector="true" Pos="8,4" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Comparator { state } => {
                assert!((state.out_high() - 3.3).abs() < 1e-9);
                assert!((state.out_low() - 0.2).abs() < 1e-9);
                assert!((state.out_imp() - 20.0).abs() < 1e-9);
                assert!(state.inverted);
                assert!(state.open_col);
            }
            other => panic!("expected Comparator, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_test_unit() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="TestUnit" CircId="TestUnit-1" Inputs="A,B" Outputs="Y" Period="100 ns" Truth="0,0,0,1," Pos="0,0" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::TestUnit(t) => {
                assert_eq!(t.drive.len(), 2);
                assert_eq!(t.sense.len(), 1);
                assert!((t.period - 100e-9).abs() < 1e-15, "period {}", t.period);
                assert_eq!(t.truth, vec![0, 0, 0, 1]);
                assert_eq!(t.drive[0].id, "TestUnit-1-out0");
                assert_eq!(t.sense[0].id, "TestUnit-1-in0");
            }
            other => panic!("expected TestUnit, got {other:?}"),
        }
    }

    #[test]
    fn parse_and_gate() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="And Gate" CircId="And Gate-1" Num_Inputs="3" Tpd_ps="20 ns" initHigh="true" Pos="8,4" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Gate(g) => {
                assert_eq!(g.inputs.len(), 3);
                assert!(g.init_high);
                assert!((g.family.delay_base - 20_000.0).abs() < 1.0);
            }
            other => panic!("expected And Gate, got {other:?}"),
        }
    }

    #[test]
    fn parse_volt_reg() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="VoltReg" CircId="VoltReg-1" Voltage="5 V" Pos="16,8" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::VoltReg { state } => {
                assert!((state.v_ref - 5.0).abs() < 1e-9);
            }
            other => panic!("expected VoltReg, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((16.0, 8.0)));
    }

    #[test]
    fn parse_instruments() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Probe" CircId="Probe-1" Threshold="1.2 V" Small="true" Pos="0,0" />
<item itemtype="Voltimeter" CircId="Voltimeter-1" RMS="true" Pos="8,0" />
<item itemtype="Amperimeter" CircId="Amperimeter-1" RMS="false" Pos="16,0" />
<item itemtype="Oscope" CircId="Oscope-1" connectGnd="false" Pos="32,0" />
<item itemtype="LAnalizer" CircId="LAnalizer-1" Pos="64,0" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert!(p.skipped.is_empty());
        assert_eq!(p.items.len(), 5);
        match &p.items[0].comp.kind {
            Kind::Probe { threshold, small } => {
                assert!((threshold - 1.2).abs() < 1e-9);
                assert!(small);
            }
            other => panic!("expected Probe, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::Voltmeter { rms, .. } => assert!(rms),
            other => panic!("expected Voltmeter, got {other:?}"),
        }
        match &p.items[3].comp.kind {
            Kind::Oscope { connect_gnd, .. } => assert!(!connect_gnd),
            other => panic!("expected Oscope, got {other:?}"),
        }
        match &p.items[4].comp.kind {
            Kind::LAnalizer { connect_gnd, .. } => assert!(connect_gnd),
            other => panic!("expected LAnalizer, got {other:?}"),
        }
    }

    #[test]
    fn parse_circuit_header() {
        let src = r#"<circuit version="2.0.0" stepSize="1000" stepsPS="500000" NLsteps="50" reaStep="2000" animate="1" anicurr="0" ansi="1" width="1600" height="1200" >
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert_eq!(p.circ.step_size, 1000);
        assert_eq!(p.circ.steps_ps, 500_000);
        assert_eq!(p.circ.nl_steps, 50);
        assert_eq!(p.circ.react_step_ps, 2000);
        assert!(p.circ.animate_logic);
        assert!(!p.circ.animate_curr);
        assert!(p.circ.ansi);
        assert_eq!(p.circ.width, 1600);
        assert_eq!(p.circ.height, 1200);
        assert!((p.analog_dt - 2e-9).abs() < 1e-20);
    }

    #[test]
    fn parse_mcu_item() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="pic14test-1" Frequency="4 MHz" ForceFreq="true" Program="blink.hex" Auto_Load="true" savePGM="false" pgm="" Pos="8,4" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert!(p.skipped.is_empty());
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::McuItem(spec) => {
                assert_eq!(spec.device, "pic14test");
                assert!((spec.frequency.unwrap() - 4e6).abs() < 1.0);
                assert!(spec.force_freq);
                assert_eq!(spec.program.as_deref(), Some("blink.hex"));
                assert!(spec.auto_load);
                assert!(!spec.save_pgm);
                assert!(spec.pgm.is_none());
            }
            other => panic!("expected McuItem, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_mcu_old_mhz() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="p16f84-1" Mhz="16" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        match &p.items[0].comp.kind {
            Kind::McuItem(spec) => {
                assert_eq!(spec.device, "p16F84");
                assert!((spec.frequency.unwrap() - 16e6).abs() < 1.0);
            }
            other => panic!("expected McuItem, got {other:?}"),
        }
    }

    #[test]
    fn parse_qemu_device() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="QemuDevice" CircId="Esp32-1" Program="app.bin" Args="-d,in_asm" Pos="8,4" />
<item itemtype="QemuDevice" CircId="STM32F103C8-1" Program="fw.bin" />
</circuit>
"#;
        let p = parse_sim1(src).unwrap();
        assert!(p.skipped.is_empty(), "{:?}", p.skipped);
        assert_eq!(p.items.len(), 2);
        match &p.items[0].comp.kind {
            Kind::QemuDevice(q) => {
                assert_eq!(q.family, crate::qemu::QemuFamily::Esp32);
                assert_eq!(q.firmware, "app.bin");
                assert_eq!(q.extra_args, "-d,in_asm");
            }
            other => panic!("expected QemuDevice Esp32, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::QemuDevice(q) => {
                assert_eq!(q.family, crate::qemu::QemuFamily::Stm32);
                assert_eq!(q.firmware, "fw.bin");
            }
            other => panic!("expected QemuDevice STM32, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_legacy_to_circ1_conversion() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Fixed Voltage" CircId="Fixed-1" Pos="0,0" Voltage="5 V" />
<item itemtype="Voltimeter" CircId="Voltimeter-1" Pos="100,0" Show_id="true" />
<item itemtype="Connector" uid="c1" startpinid="Fixed-1-outnod" endpinid="Voltimeter-1-lPin" />
</circuit>
"#;
        let p = parse_legacy_to_circ1(src).unwrap();
        assert_eq!(p.version, crate::circ1::FORMAT_VERSION);
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].itemtype, "FixedVolt");
        assert_eq!(p.items[1].itemtype, "Voltmeter");
        assert!(p.items[1].graphic.show_id);
        assert_eq!(p.connectors.len(), 1);
        assert_eq!(p.connectors[0].id, "c1");
    }
}
