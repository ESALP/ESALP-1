#!/usr/bin/env perl
# Split the input into different tests on the `EOT` string
mkdir "test";
my $cur = 0;
my $out;
my $filename;
my $line;
sub openfile {
	$cur += 1;
	$filename = "test/".$cur.".t";
	open ($out, ">", $filename) or die "Can't open $filename: $!";
}
openfile;
while (<>) {
	if (/EOT/) {
		close $out or die "Cannot close $filename: $!";
		openfile;
	} else {
		print $out $_;
	}
	$last = $_;
}
# Delete the extra file
close $out or die "Cannot close $filename: $!";
if ($last =~ /EOT/) {
	unlink $filename or die "Cannot delete $filename: $!";
}
