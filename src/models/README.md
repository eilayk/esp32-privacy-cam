model obtained from https://github.com/PINTO0309/PINTO_model_zoo

Model Layers
+-------+-------------------+--------+--------+-------------------------+--------------+------------------------------------------------------+
| Index | OpCode            | # Ops  | # MACs | Input Shape             | Output Shape | Options                                              |
+-------+-------------------+--------+--------+-------------------------+--------------+------------------------------------------------------+
| 0     | quantize          | 196.6k | 0      | 1x128x128x3             | 1x128x128x3  | Type=none                                            |
| 1     | conv_2d           | 15.0M  | 7.4M   | 1x128x128x3,24x5x5x3,24 | 1x64x64x24   | Padding:Same stride:2x2 activation:Relu              |
| 2     | depthwise_conv_2d | 1.9M   | 884.7k | 1x64x64x24,1x3x3x24,24  | 1x64x64x24   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 3     | conv_2d           | 4.8M   | 2.4M   | 1x64x64x24,24x1x1x24,24 | 1x64x64x24   | Padding:Valid stride:1x1 activation:None             |
| 4     | add               | 98.3k  | 0      | 1x64x64x24,1x64x64x24   | 1x64x64x24   | Activation:Relu                                      |
| 5     | pad               | 688.1k | 0      | 1x64x64x24,4x2          | 1x64x64x28   | Type=padoptions                                      |
| 6     | depthwise_conv_2d | 1.9M   | 884.7k | 1x64x64x24,1x3x3x24,24  | 1x64x64x24   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 7     | conv_2d           | 5.6M   | 2.8M   | 1x64x64x24,28x1x1x24,28 | 1x64x64x28   | Padding:Valid stride:1x1 activation:None             |
| 8     | add               | 114.7k | 0      | 1x64x64x28,1x64x64x28   | 1x64x64x28   | Activation:Relu                                      |
| 9     | depthwise_conv_2d | 544.8k | 258.0k | 1x64x64x28,1x3x3x28,28  | 1x32x32x28   | Multiplier:1 padding:Same stride:2x2 activation:None |
| 10    | conv_2d           | 1.9M   | 917.5k | 1x32x32x28,32x1x1x28,32 | 1x32x32x32   | Padding:Valid stride:1x1 activation:None             |
| 11    | max_pool_2d       | 114.7k | 0      | 1x64x64x28              | 1x32x32x28   | Padding:Same stride:2x2 filter:2x2 activation:None   |
| 12    | pad               | 196.6k | 0      | 1x32x32x28,4x2          | 1x32x32x32   | Type=padoptions                                      |
| 13    | add               | 32.8k  | 0      | 1x32x32x32,1x32x32x32   | 1x32x32x32   | Activation:Relu                                      |
| 14    | pad               | 221.2k | 0      | 1x32x32x32,4x2          | 1x32x32x36   | Type=padoptions                                      |
| 15    | depthwise_conv_2d | 622.6k | 294.9k | 1x32x32x32,1x3x3x32,32  | 1x32x32x32   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 16    | conv_2d           | 2.4M   | 1.2M   | 1x32x32x32,36x1x1x32,36 | 1x32x32x36   | Padding:Valid stride:1x1 activation:None             |
| 17    | add               | 36.9k  | 0      | 1x32x32x36,1x32x32x36   | 1x32x32x36   | Activation:Relu                                      |
| 18    | pad               | 258.0k | 0      | 1x32x32x36,4x2          | 1x32x32x42   | Type=padoptions                                      |
| 19    | depthwise_conv_2d | 700.4k | 331.8k | 1x32x32x36,1x3x3x36,36  | 1x32x32x36   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 20    | conv_2d           | 3.1M   | 1.5M   | 1x32x32x36,42x1x1x36,42 | 1x32x32x42   | Padding:Valid stride:1x1 activation:None             |
| 21    | add               | 43.0k  | 0      | 1x32x32x42,1x32x32x42   | 1x32x32x42   | Activation:Relu                                      |
| 22    | depthwise_conv_2d | 204.3k | 96.8k  | 1x32x32x42,1x3x3x42,42  | 1x16x16x42   | Multiplier:1 padding:Same stride:2x2 activation:None |
| 23    | conv_2d           | 1.0M   | 516.1k | 1x16x16x42,48x1x1x42,48 | 1x16x16x48   | Padding:Valid stride:1x1 activation:None             |
| 24    | max_pool_2d       | 43.0k  | 0      | 1x32x32x42              | 1x16x16x42   | Padding:Same stride:2x2 filter:2x2 activation:None   |
| 25    | pad               | 73.7k  | 0      | 1x16x16x42,4x2          | 1x16x16x48   | Type=padoptions                                      |
| 26    | add               | 12.3k  | 0      | 1x16x16x48,1x16x16x48   | 1x16x16x48   | Activation:Relu                                      |
| 27    | pad               | 86.0k  | 0      | 1x16x16x48,4x2          | 1x16x16x56   | Type=padoptions                                      |
| 28    | depthwise_conv_2d | 233.5k | 110.6k | 1x16x16x48,1x3x3x48,48  | 1x16x16x48   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 29    | conv_2d           | 1.4M   | 688.1k | 1x16x16x48,56x1x1x48,56 | 1x16x16x56   | Padding:Valid stride:1x1 activation:None             |
| 30    | add               | 14.3k  | 0      | 1x16x16x56,1x16x16x56   | 1x16x16x56   | Activation:Relu                                      |
| 31    | pad               | 98.3k  | 0      | 1x16x16x56,4x2          | 1x16x16x64   | Type=padoptions                                      |
| 32    | depthwise_conv_2d | 272.4k | 129.0k | 1x16x16x56,1x3x3x56,56  | 1x16x16x56   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 33    | conv_2d           | 1.9M   | 917.5k | 1x16x16x56,64x1x1x56,64 | 1x16x16x64   | Padding:Valid stride:1x1 activation:None             |
| 34    | add               | 16.4k  | 0      | 1x16x16x64,1x16x16x64   | 1x16x16x64   | Activation:Relu                                      |
| 35    | pad               | 110.6k | 0      | 1x16x16x64,4x2          | 1x16x16x72   | Type=padoptions                                      |
| 36    | depthwise_conv_2d | 311.3k | 147.5k | 1x16x16x64,1x3x3x64,64  | 1x16x16x64   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 37    | conv_2d           | 2.4M   | 1.2M   | 1x16x16x64,72x1x1x64,72 | 1x16x16x72   | Padding:Valid stride:1x1 activation:None             |
| 38    | add               | 18.4k  | 0      | 1x16x16x72,1x16x16x72   | 1x16x16x72   | Activation:Relu                                      |
| 39    | pad               | 122.9k | 0      | 1x16x16x72,4x2          | 1x16x16x80   | Type=padoptions                                      |
| 40    | depthwise_conv_2d | 350.2k | 165.9k | 1x16x16x72,1x3x3x72,72  | 1x16x16x72   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 41    | conv_2d           | 3.0M   | 1.5M   | 1x16x16x72,80x1x1x72,80 | 1x16x16x80   | Padding:Valid stride:1x1 activation:None             |
| 42    | add               | 20.5k  | 0      | 1x16x16x80,1x16x16x80   | 1x16x16x80   | Activation:Relu                                      |
| 43    | pad               | 135.2k | 0      | 1x16x16x80,4x2          | 1x16x16x88   | Type=padoptions                                      |
| 44    | depthwise_conv_2d | 389.1k | 184.3k | 1x16x16x80,1x3x3x80,80  | 1x16x16x80   | Multiplier:1 padding:Same stride:1x1 activation:None |
| 45    | conv_2d           | 3.6M   | 1.8M   | 1x16x16x80,88x1x1x80,88 | 1x16x16x88   | Padding:Valid stride:1x1 activation:None             |
| 46    | add               | 22.5k  | 0      | 1x16x16x88,1x16x16x88   | 1x16x16x88   | Activation:Relu                                      |
| 47    | conv_2d           | 90.6k  | 45.1k  | 1x16x16x88,2x1x1x88,2   | 1x16x16x2    | Padding:Same stride:1x1 activation:None              |
| 48    | depthwise_conv_2d | 107.0k | 50.7k  | 1x16x16x88,1x3x3x88,88  | 1x8x8x88     | Multiplier:1 padding:Same stride:2x2 activation:None |
| 49    | conv_2d           | 1.1M   | 540.7k | 1x8x8x88,96x1x1x88,96   | 1x8x8x96     | Padding:Valid stride:1x1 activation:None             |
| 50    | max_pool_2d       | 22.5k  | 0      | 1x16x16x88              | 1x8x8x88     | Padding:Same stride:2x2 filter:2x2 activation:None   |
| 51    | pad               | 36.9k  | 0      | 1x8x8x88,4x2            | 1x8x8x96     | Type=padoptions                                      |
| 52    | add               | 6.1k   | 0      | 1x8x8x96,1x8x8x96       | 1x8x8x96     | Activation:Relu                                      |
| 53    | depthwise_conv_2d | 116.7k | 55.3k  | 1x8x8x96,1x3x3x96,96    | 1x8x8x96     | Multiplier:1 padding:Same stride:1x1 activation:None |
| 54    | conv_2d           | 1.2M   | 589.8k | 1x8x8x96,96x1x1x96,96   | 1x8x8x96     | Padding:Valid stride:1x1 activation:None             |
| 55    | add               | 6.1k   | 0      | 1x8x8x96,1x8x8x96       | 1x8x8x96     | Activation:Relu                                      |
| 56    | depthwise_conv_2d | 116.7k | 55.3k  | 1x8x8x96,1x3x3x96,96    | 1x8x8x96     | Multiplier:1 padding:Same stride:1x1 activation:None |
| 57    | conv_2d           | 1.2M   | 589.8k | 1x8x8x96,96x1x1x96,96   | 1x8x8x96     | Padding:Valid stride:1x1 activation:None             |
| 58    | add               | 6.1k   | 0      | 1x8x8x96,1x8x8x96       | 1x8x8x96     | Activation:Relu                                      |
| 59    | depthwise_conv_2d | 116.7k | 55.3k  | 1x8x8x96,1x3x3x96,96    | 1x8x8x96     | Multiplier:1 padding:Same stride:1x1 activation:None |
| 60    | conv_2d           | 1.2M   | 589.8k | 1x8x8x96,96x1x1x96,96   | 1x8x8x96     | Padding:Valid stride:1x1 activation:None             |
| 61    | add               | 6.1k   | 0      | 1x8x8x96,1x8x8x96       | 1x8x8x96     | Activation:Relu                                      |
| 62    | depthwise_conv_2d | 116.7k | 55.3k  | 1x8x8x96,1x3x3x96,96    | 1x8x8x96     | Multiplier:1 padding:Same stride:1x1 activation:None |
| 63    | conv_2d           | 1.2M   | 589.8k | 1x8x8x96,96x1x1x96,96   | 1x8x8x96     | Padding:Valid stride:1x1 activation:None             |
| 64    | add               | 6.1k   | 0      | 1x8x8x96,1x8x8x96       | 1x8x8x96     | Activation:Relu                                      |
| 65    | conv_2d           | 74.1k  | 36.9k  | 1x8x8x96,6x1x1x96,6     | 1x8x8x6      | Padding:Same stride:1x1 activation:None              |
| 66    | conv_2d           | 1.2M   | 589.8k | 1x8x8x96,96x1x1x96,96   | 1x8x8x96     | Padding:Same stride:1x1 activation:None              |
| 67    | conv_2d           | 1.4M   | 720.9k | 1x16x16x88,32x1x1x88,32 | 1x16x16x32   | Padding:Same stride:1x1 activation:None              |
| 68    | reshape           | 0      | 0      | 1x16x16x2,3             | 1x512x1      | Type=none                                            |
| 69    | reshape           | 0      | 0      | 1x16x16x32,3            | 1x512x16     | Type=none                                            |
| 70    | reshape           | 0      | 0      | 1x8x8x6,3               | 1x384x1      | Type=none                                            |
| 71    | quantize          | 2.0k   | 0      | 1x512x1                 | 1x512x1      | Type=none                                            |
| 72    | concatenation     | 0      | 0      | 1x512x1,1x384x1         | 1x896x1      | Type=concatenationoptions                            |
| 73    | reshape           | 0      | 0      | 1x8x8x96,3              | 1x384x16     | Type=none                                            |
| 74    | quantize          | 32.8k  | 0      | 1x512x16                | 1x512x16     | Type=none                                            |
| 75    | concatenation     | 0      | 0      | 1x512x16,1x384x16       | 1x896x16     | Type=concatenationoptions                            |
| 76    | dequantize        | 1.8k   | 0      | 1x896x1                 | 1x896x1      | Type=none                                            |
| 77    | dequantize        | 28.7k  | 0      | 1x896x16                | 1x896x16     | Type=none                                            |
+-------+-------------------+--------+--------+-------------------------+--------------+-------------------------------------------------