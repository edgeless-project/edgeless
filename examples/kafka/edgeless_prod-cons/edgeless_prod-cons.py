import argparse
import matplotlib.pyplot as plt


def parse_timestamp(timestamp):
    seconds, nanoseconds = timestamp.split('.')
    return int(seconds) 

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

def save_differences(differences, output_file):
    with open(output_file, 'w') as file:
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

    producer_file = '../edgeless.prod'
    consumer_file = '../consumer/edgeless.cons'

    producer_lines = read_file(producer_file)
    consumer_lines = read_file(consumer_file)[1:] #skip the first line


    producer_timestamps = [line.strip().split(' ')[0] for line in producer_lines]
    consumer_timestamps = [line.strip().split(' - Payload: ')[0] for line in consumer_lines]

    if len(producer_timestamps) != len(consumer_timestamps):
        print("Warning: Producer and consumer files have different lengths")

    differences = calculate_differences(producer_timestamps, consumer_timestamps)
    save_differences(differences, args.output)
    plot_histogram(differences, args.output)

if __name__ == '__main__':
    main()
