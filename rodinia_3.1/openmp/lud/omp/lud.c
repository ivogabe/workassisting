/*
 * =====================================================================================
 *
 *       Filename:  suite.c
 *
 *    Description:  The main wrapper for the suite
 *
 *        Version:  1.0
 *        Created:  10/22/2009 08:40:34 PM
 *       Revision:  none
 *       Compiler:  gcc
 *
 *         Author:  Liang Wang (lw2aw), lw2aw@virginia.edu
 *        Company:  CS@UVa
 * Adapted by Ivo Gabe de Wolff
 * =====================================================================================
 */

#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <getopt.h>
#include <stdlib.h>
#include <assert.h>
#include <omp.h>

#include "common.h"

static int do_verify = 0;
int omp_num_threads = 40;

static struct option long_options[] = {
  /* name, has_arg, flag, val */
  {"input", 1, NULL, 'i'},
  {"size", 1, NULL, 's'},
  {"matrixcount", 1, NULL, 'm'},
  {"verify", 0, NULL, 'v'},
  {0,0,0,0}
};

extern void
lud_omp(float *m, int matrix_dim);

int
main ( int argc, char *argv[] )
{
  int matrix_dim = 32; /* default size */
  int matrix_count = 1;
  int opt, option_index=0;
  func_ret_t ret;
  const char *input_file = NULL;
  float *m, *mm;
  stopwatch sw;

  while ((opt = getopt_long(argc, argv, "::vs:n:i:m:", 
                            long_options, &option_index)) != -1 ) {
    switch(opt){
    case 'i':
      input_file = optarg;
      break;
    case 'v':
      do_verify = 1;
      break;
    case 'm':
      matrix_count = atoi(optarg);
      break;
    case 'n':
      omp_num_threads = atoi(optarg);
      break;
    case 's':
      matrix_dim = atoi(optarg);
      printf("Generate input matrix internally, size =%d\n", matrix_dim);
      // fprintf(stderr, "Currently not supported, use -i instead\n");
      // fprintf(stderr, "Usage: %s [-v] [-s matrix_size|-i input_file]\n", argv[0]);
      // exit(EXIT_FAILURE);
      break;
    case '?':
      fprintf(stderr, "invalid option\n");
      break;
    case ':':
      fprintf(stderr, "missing argument\n");
      break;
    default:
      fprintf(stderr, "Usage: %s [-v] [-s matrix_size|-i input_file]\n",
	      argv[0]);
      exit(EXIT_FAILURE);
    }
  }
  
  if ( (optind < argc) || (optind == 1)) {
    fprintf(stderr, "Usage: %s [-v] [-n no. of threads] [-s matrix_size|-i input_file] [-m matrix_count]\n", argv[0]);
    exit(EXIT_FAILURE);
  }

  if (input_file) {
    printf("Reading matrix from file %s\n", input_file);
    ret = create_matrix_from_file(&m, input_file, &matrix_dim);
    if (ret != RET_SUCCESS) {
      m = NULL;
      fprintf(stderr, "error create matrix from file %s\n", input_file);
      exit(EXIT_FAILURE);
    }
  }
  else if (matrix_dim) {
    printf("Creating matrix internally size=%d\n", matrix_dim);
    ret = create_matrix(&m, matrix_dim);
    if (ret != RET_SUCCESS) {
      m = NULL;
      fprintf(stderr, "error create matrix internally size=%d\n", matrix_dim);
      exit(EXIT_FAILURE);
    }
  }
 
  else {
    printf("No input file specified!\n");
    exit(EXIT_FAILURE);
  }

  if (matrix_count < 1) {
    printf("Matrix count should be at least 1.\n");
    exit(EXIT_FAILURE);
  }

  if (do_verify){
    printf("Before LUD\n");
    /* print_matrix(m, matrix_dim); */
    matrix_duplicate(m, &mm, matrix_dim);
  }

  float** matrices = malloc(sizeof(float*) * matrix_count);
  for (int i = 0; i < matrix_count; i++) {
    matrix_duplicate(m, &matrices[i], matrix_dim);
  }

  // Warm-up run

  if (matrix_count == 1) {
    lud_omp(matrices[0], matrix_dim);
  } else {
    omp_set_max_active_levels(2);
    omp_set_num_threads(matrix_count);
    #pragma omp parallel for
    for (int i = 0; i < matrix_count; i++) {
      omp_set_num_threads(omp_num_threads);
      lud_omp(matrices[i], matrix_dim);
    }
  }

  for (int i = 0; i < matrix_count; i++) {
    memcpy(matrices[i], m, matrix_dim * matrix_dim * sizeof(float));
  }

  if (matrix_count == 1) {
    stopwatch_start(&sw);
    lud_omp(matrices[0], matrix_dim);
    stopwatch_stop(&sw);
  } else {
    omp_set_max_active_levels(2);
    omp_set_num_threads(matrix_count);

    stopwatch_start(&sw);

    #pragma omp parallel for
    for (int i = 0; i < matrix_count; i++) {
      omp_set_num_threads(omp_num_threads);
      lud_omp(matrices[i], matrix_dim);
    }

    stopwatch_stop(&sw);
  }

  printf("Time consumed(ms): %lf\n", 1000*get_interval_by_sec(&sw));

  if (do_verify){
    printf("After LUD\n");
    /* print_matrix(m, matrix_dim); */
    printf(">>>Verify<<<<\n");
    lud_verify(mm, m, matrix_dim); 
    free(mm);
  }
  
  free(m);

  return EXIT_SUCCESS;
}				/* ----------  end of function main  ---------- */
