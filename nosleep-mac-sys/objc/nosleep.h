#import <Foundation/Foundation.h>
#import <IOKit/pwr_mgt/IOPMLib.h>

CFStringRef assertionName = CFSTR("Power Save Blocker");

int start(NSString *noSleepType, UInt32 *handle);

void stop(UInt32 handle);

bool isActive();