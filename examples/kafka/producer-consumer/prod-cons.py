import argparse
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def parse_timestamp(timestamp):
    seconds, nanoseconds = timestamp.split('.')
    return int(seconds) + int(nanoseconds) / 1e9

def read_file(file_path):
    with open(file_path, 'r') as file:
        lines = file.readlines()
    return lines

def calculate_differences(producer_timestamps, consumer_timestamps):
    differences = []
    for prod_ts, cons_ts in zip(producer_timestamps, consumer_timestamps):
        prod_time = parse_timestamp(prod_ts)
        cons_time = parse_timestamp(cons_ts)
        difference = cons_time - prod_time
        differences.append(difference)
    return differences

def save_differences(differences, output_file, producer_first_line, consumer_first_line, producer_first_timestamp):
    with open(output_file, 'w') as file:
        file.write(producer_first_line)
        file.write(consumer_first_line)
        file.write("# "+ producer_first_timestamp)
        for diff in differences:
            file.write(f'{diff}\n')

#creating an histogram
def plot_histogram(differences, output_file):
    plt.figure(figsize=(10, 6))
    
    plt.hist(differences, bins=100, color='blue', edgecolor='black')
    plt.title('Timestamp Differences between Producer and Consumer')
    plt.xlabel('Latency (seconds)')
    plt.ylabel('Frequency')
    plt.grid(True)
    histogram_file = output_file.replace('.txt', '.png')
    plt.savefig(histogram_file)
    print(f'Histogram saved as {histogram_file}')

def main():
    parser = argparse.ArgumentParser(description='Calculate timestamp differences between producer and consumer')
    parser.add_argument('--output', required=True, help='Output file for the differences')
    args = parser.parse_args()

    producer_file = '../producer/bench.prod'
    consumer_file = '../consumer/bench.cons'

    producer_lines = read_file(producer_file)
    consumer_lines = read_file(consumer_file)

    producer_first_line = producer_lines[0]
    consumer_first_line = consumer_lines[0]

    producer_timestamps = [line.strip().split(' - Payload: ')[0] for line in producer_lines[1:]]
    consumer_timestamps = [line.strip().split(' - Payload: ')[0] for line in consumer_lines[1:]]

    if len(producer_timestamps) != len(consumer_timestamps):
        print("Warning: Producer and consumer files have different lengths")

    differences = calculate_differences(producer_timestamps, consumer_timestamps)
    save_differences(differences, args.output, producer_first_line, consumer_first_line, producer_timestamps[0] + '\n')
    plot_histogram(differences, args.output)

if __name__ == '__main__':
    main()
