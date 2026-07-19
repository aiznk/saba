#include <stdio.h>

struct RTAB1 {
	int id;
};

struct RTAB2 {
	int id;
	int rtab2_id;
};

int main(void) {
	struct RTAB2 rtab2s[3] = {
		{ 1, 1 },
		{ 2, 1 },
		{ 3, 2 },
	};
	struct RTAB1 rtab1s[5] = {
		{ 1 },
		{ 2 },
		{ 3 },
		{ 4 },
		{ 5 },
	};
	int matched[5] = {0};

	for (int i = 0; i < 3; i++) {
		struct RTAB2 *rtab2 = &rtab2s[i];

		for (int j = 0; j < 5; j++) {
			struct RTAB1 *rtab1 = &rtab1s[j];

			if (rtab2->rtab2_id == rtab1->id) {
				printf("%d,%d\n",rtab2->id, rtab1->id);
				matched[j] = 1;
			}
		}
	}
	for (int k = 0; k < 5; k++) {
		struct RTAB1 *rtab1 = &rtab1s[k];
		if (!matched[k]) {
			printf("$nil,%d\n", rtab1->id);
		}
	}

	return 0;
}
