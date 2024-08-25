# import os
import platform
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.font_manager as fm
import seaborn as sns
from scipy import stats


def read_and_prepare_data(file_path):
    """从文件读取数据并直接构建DataFrame"""
    with open(file_path, 'r') as file:
        lines = file.readlines()
        successes = list(map(int, lines[0].strip('[]\n').split(', ')))
        attempts = list(map(int, lines[1].strip('[]\n').split(', ')))
        times = list(map(float, lines[2].strip('[]\n').split(', ')))

    data = {
        'Experiment': [f'Exp{i + 1}' for i in range(len(successes))],
        'Successes': successes,
        'Attempts': attempts,
        'TimeTaken': times
    }
    return pd.DataFrame(data)


def analyze_experiments(df):
    """综合分析实验数据，包括成功率统计、分布、与时间的关系等"""
    df['SuccessRate'] = df['Successes'] / df['Attempts']

    print(f"平均成功率: {df['SuccessRate'].mean() * 100:.2f}%")
    print(f"标准差: {df['SuccessRate'].std()}")

    # 绘制成功率分布图
    sns.histplot(df['SuccessRate'], kde=True)
    plt.title('成功率分布')
    plt.xlabel('成功率')
    plt.ylabel('频率')
    plt.show()

    # 绘制箱线图
    sns.boxplot(y='SuccessRate', data=df)
    plt.title('成功率分布（箱线图）')
    plt.ylabel('成功率')
    plt.show()

    # 绘制尝试次数与成功率的关系图
    plt.figure(figsize=(10, 6))
    sns.scatterplot(x='Attempts', y='SuccessRate', data=df, alpha=0.6)
    plt.title('尝试次数与成功率的关系')
    plt.xlabel('尝试次数')
    plt.ylabel('成功率')
    plt.show()

    # 分析时间与成功率的关系
    sns.scatterplot(x='TimeTaken', y='SuccessRate', data=df, alpha=0.6)
    plt.title('实验时间与成功率的关系')
    plt.xlabel('实验时间 (单位:秒)')
    plt.ylabel('成功率')
    plt.show()

    print("实验时间与成功率的相关性分析:")
    corr = df['TimeTaken'].corr(df['SuccessRate'])
    print(f"相关系数: {corr}")

    # t检验
    t_statistic, p_value = stats.ttest_1samp(df['SuccessRate'], popmean=df['SuccessRate'].mean())
    print(f"t检验结果: t-statistic={t_statistic}, p-value={p_value}")
    significance = "存在显著差异" if p_value < 0.05 else "没有足够证据表明存在显著差异"
    print(f"关于成功率的假设检验结果表明：{significance}")


def set_chinese_font():
    """设置中文字体"""
    # 检测是否为Windows系统
    if platform.system() == 'Windows':
        # 设置SimHei字体（需要系统中已安装）
        plt.rcParams['font.sans-serif'] = ['SimHei']
    else:
        # 对于非Windows系统，推荐使用Noto Sans CJK SC
        noto_font_path = fm.findfont('Noto Sans CJK SC')
        plt.rcParams['font.family'] = ['Noto Sans CJK SC']
        # 如果字体不在标准路径，可能需要显式添加字体路径
        # plt.rcParams['font.path'] = noto_font_path

    # 确保matplotlib能正确显示中文标签
    plt.rcParams['axes.unicode_minus'] = False

def main():
    file_path = '../measure/result.txt'
    set_chinese_font()
    df = read_and_prepare_data(file_path)
    analyze_experiments(df)

    # 确保数据处理无误后才删除文件
    # os.remove(file_path)

if __name__ == "__main__":
    main()
